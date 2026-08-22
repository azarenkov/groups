use std::{error::Error, sync::Arc, time::Duration};

use platform::ports::outbox_publisher::OutboxPublisher;
use platform::{
    config::{nats::NatsConfig, postgres::PostgresConfig},
    database::postgres::connection::{create_postgres_pool, ping_pool},
    messaging::nats::{
        connection::{create_jetstream, is_nats_connected},
        producer::NatsOutboxPublisher,
    },
    outbox::dispatcher::OutboxDispatcher,
};
use shared::health::{self, Readiness};
use tokio::signal;

use crate::config::WorkerConfig;

mod config;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    shared::observability::init("outbox-worker");

    let worker_config = WorkerConfig::from_env()?;
    let postgres_config = PostgresConfig::from_env()?;
    let nats_config = NatsConfig::from_env()?;

    let pool = create_postgres_pool(postgres_config).await?;
    let jetstream = create_jetstream(&nats_config).await?;

    let readiness = Readiness::builder()
        .add_probe({
            let pool = pool.clone();
            move || {
                let pool = pool.clone();
                async move { ping_pool(&pool).await }
            }
        })
        .add_probe({
            let jetstream = jetstream.clone();
            move || {
                let jetstream = jetstream.clone();
                async move { is_nats_connected(&jetstream) }
            }
        })
        .build();
    let health_server = health::bind(worker_config.health_port, readiness.clone())?;
    let health_handle = actix_web::rt::spawn(async move {
        if let Err(e) = health_server.await {
            tracing::error!(error = %e, "Health server stopped");
        }
    });

    let publisher = Arc::new(NatsOutboxPublisher::new(
        jetstream,
        nats_config.subject_prefix.clone(),
        nats_config.event_partitions,
    ));
    let dispatcher = Arc::new(OutboxDispatcher::new(
        pool,
        publisher,
        worker_config.outbox_max_attempts,
        worker_config.outbox_backoff_base_secs,
        worker_config.outbox_backoff_max_multiplier,
    ));

    let idle_interval = Duration::from_millis(worker_config.outbox_idle_interval_ms);
    let batch_size = worker_config.outbox_batch_size;
    let cleanup_interval = Duration::from_secs(worker_config.outbox_cleanup_interval_secs);
    let cleanup_retention = worker_config.outbox_cleanup_retention_secs;
    let cleanup_batch_size = worker_config.outbox_cleanup_batch_size;

    tracing::info!(
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

    let cleanup_handle = tokio::spawn(run_cleanup_loop(
        dispatcher.clone(),
        cleanup_interval,
        cleanup_retention,
        cleanup_batch_size,
    ));

    let shutdown = signal::ctrl_c();
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            _ = &mut shutdown => {
                tracing::info!("Shutdown signal received, exiting");
                break;
            }
            result = run_batch(&dispatcher, batch_size) => {
                match result {
                    Ok(0) => {
                        tokio::select! {
                            _ = &mut shutdown => {
                                tracing::info!("Shutdown signal received during idle, exiting");
                                break;
                            }
                            _ = tokio::time::sleep(idle_interval) => {}
                        }
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!(error = %e, "Outbox batch failed, backing off");
                        tokio::select! {
                            _ = &mut shutdown => break,
                            _ = tokio::time::sleep(idle_interval) => {}
                        }
                    }
                }
            }
        }
    }

    cleanup_handle.abort();
    health_handle.abort();

    Ok(())
}

async fn run_batch<P>(
    dispatcher: &OutboxDispatcher<P>,
    batch_size: u32,
) -> Result<usize, Box<dyn Error>>
where
    P: OutboxPublisher,
{
    let mut processed = 0usize;
    for _ in 0..batch_size {
        if !dispatcher.process_one().await? {
            break;
        }
        processed += 1;
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
