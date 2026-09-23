use std::{error::Error, sync::Arc, time::Duration};

use ports::outbox_publisher::OutboxPublisher;
use infrastructure_common::{
    config::{nats::NatsConfig, postgres::PostgresConfig},
    database::postgres::connection::{create_postgres_pool, ping_pool},
    messaging::nats::{
        connection::{create_jetstream, is_nats_connected},
        producer::NatsOutboxPublisher,
    },
    outbox::dispatcher::OutboxDispatcher,
};
use api_common::health::{self, Heartbeat, Liveness, Readiness};
use api_common::shutdown::wait_for_signal;
use tokio::sync::watch;
use tracing::Instrument;

use crate::config::WorkerConfig;

mod config;

struct Relay<P: OutboxPublisher> {
    source: String,
    dispatcher: Arc<OutboxDispatcher<P>>,
    heartbeat: Heartbeat,
}

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    if let Some(code) = api_common::health::run_probe_from_args() {
        std::process::exit(code);
    }
    infrastructure_common::observability::init("outbox-worker");

    let worker_config = WorkerConfig::from_env()?;
    let sources = worker_config.sources()?;
    let nats_config = NatsConfig::from_env()?;

    let jetstream = create_jetstream(&nats_config).await?;
    let publisher = Arc::new(NatsOutboxPublisher::new(
        jetstream.clone(),
        nats_config.subject_prefix.clone(),
        nats_config.event_partitions,
    ));

    let mut readiness = Readiness::builder().add_probe({
        let jetstream = jetstream.clone();
        move || {
            let jetstream = jetstream.clone();
            async move { is_nats_connected(&jetstream) }
        }
    });
    let mut liveness = Liveness::default();
    let mut relays = Vec::with_capacity(sources.len());
    for source in sources {
        let postgres_config = PostgresConfig::from_env_for(&source)?;
        let database = postgres_config.database().to_string();
        let pool = create_postgres_pool(postgres_config, &source, None).await?;
        readiness = readiness.add_probe({
            let pool = pool.clone();
            move || {
                let pool = pool.clone();
                async move { ping_pool(&pool).await }
            }
        });
        let heartbeat = Heartbeat::new(Duration::from_secs(worker_config.outbox_liveness_stale_secs));
        liveness = liveness.watch(heartbeat.clone());
        tracing::info!(%source, %database, "Outbox source connected");
        relays.push(Relay {
            source,
            dispatcher: Arc::new(OutboxDispatcher::new(
                pool,
                publisher.clone(),
                worker_config.outbox_max_attempts,
                worker_config.outbox_backoff_base_secs,
                worker_config.outbox_backoff_max_multiplier,
            )),
            heartbeat,
        });
    }
    let readiness = readiness.build();

    let health_server = health::bind(worker_config.health_port, readiness.clone(), liveness)?;
    let health_server_handle = health_server.handle();
    let health_handle = actix_web::rt::spawn(async move {
        if let Err(e) = health_server.await {
            tracing::error!(error = %e, "Health community stopped");
        }
    });

    let idle_interval = Duration::from_millis(worker_config.outbox_idle_interval_ms);
    let batch_size = worker_config.outbox_batch_size;
    let cleanup_interval = Duration::from_secs(worker_config.outbox_cleanup_interval_secs);
    let cleanup_retention = worker_config.outbox_cleanup_retention_secs;
    let cleanup_batch_size = worker_config.outbox_cleanup_batch_size;

    tracing::info!(
        sources = ?relays.iter().map(|r| r.source.as_str()).collect::<Vec<_>>(),
        batch_size,
        idle_interval_ms = worker_config.outbox_idle_interval_ms,
        max_attempts = worker_config.outbox_max_attempts,
        backoff_base_secs = worker_config.outbox_backoff_base_secs,
        backoff_max_multiplier = worker_config.outbox_backoff_max_multiplier,
        cleanup_interval_secs = worker_config.outbox_cleanup_interval_secs,
        cleanup_retention_secs = cleanup_retention,
        "Outbox worker started"
    );

    readiness.mark_ready();

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    actix_web::rt::spawn({
        let readiness = readiness.clone();
        async move {
            wait_for_signal().await;
            readiness.mark_not_ready();
            let _ = shutdown_tx.send(true);
        }
    });

    let mut cleanup_handles = Vec::with_capacity(relays.len());
    let mut relay_handles = Vec::with_capacity(relays.len());
    for relay in relays {
        let span = tracing::info_span!("outbox_source", source = %relay.source);
        cleanup_handles.push(tokio::spawn(
            run_cleanup_loop(
                relay.dispatcher.clone(),
                cleanup_interval,
                cleanup_retention,
                cleanup_batch_size,
            )
            .instrument(span.clone()),
        ));
        relay_handles.push(tokio::spawn(
            run_relay(relay, batch_size, idle_interval, shutdown_rx.clone()).instrument(span),
        ));
    }

    for handle in relay_handles {
        if let Err(e) = handle.await {
            tracing::error!(error = %e, "Outbox relay task failed");
        }
    }

    for handle in cleanup_handles {
        handle.abort();
    }
    health_server_handle.stop(false).await;
    let _ = health_handle.await;

    Ok(())
}

async fn run_relay<P>(
    relay: Relay<P>,
    batch_size: u32,
    idle_interval: Duration,
    mut shutdown_rx: watch::Receiver<bool>,
) where
    P: OutboxPublisher + Send + Sync + 'static,
{
    loop {
        relay.heartbeat.beat();
        if *shutdown_rx.borrow() {
            tracing::info!("Shutdown requested, exiting");
            break;
        }
        match run_batch(&relay.dispatcher, batch_size, &relay.heartbeat, &shutdown_rx).await {
            Ok(0) => {
                tokio::select! {
                    _ = shutdown_rx.changed() => {
                        tracing::info!("Shutdown requested during idle, exiting");
                        break;
                    }
                    _ = relay.heartbeat.keep_alive_for(idle_interval) => {}
                }
            }
            Ok(_) => {}
            Err(e) => {
                tracing::error!(error = %e, "Outbox batch failed, backing off");
                tokio::select! {
                    _ = shutdown_rx.changed() => break,
                    _ = relay.heartbeat.keep_alive_for(idle_interval) => {}
                }
            }
        }
    }
}

async fn run_batch<P>(
    dispatcher: &OutboxDispatcher<P>,
    batch_size: u32,
    heartbeat: &Heartbeat,
    shutdown: &watch::Receiver<bool>,
) -> Result<usize, Box<dyn Error + Send + Sync>>
where
    P: OutboxPublisher,
{
    let mut processed = 0usize;
    for _ in 0..batch_size {
        if *shutdown.borrow() {
            break;
        }
        if !dispatcher.process_one().await? {
            break;
        }
        processed += 1;
        heartbeat.beat();
    }
    Ok(processed)
}

async fn run_cleanup_loop<P>(
    dispatcher: Arc<OutboxDispatcher<P>>,
    interval: Duration,
    retention_secs: u64,
    batch_size: i64,
) where
    P: OutboxPublisher,
{
    loop {
        tokio::time::sleep(interval).await;

        match dispatcher
            .cleanup_processed(retention_secs, batch_size)
            .await
        {
            Ok(0) => {}
            Ok(rows) => {
                tracing::info!(rows, "Outbox cleanup deleted processed events");
            }
            Err(e) => {
                tracing::error!(error = %e, "Outbox cleanup failed");
            }
        }
    }
}
