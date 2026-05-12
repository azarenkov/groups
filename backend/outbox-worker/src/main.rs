use std::{error::Error, sync::Arc, time::Duration};

use application::ports::outbox_publisher::OutboxPublisher;
use infrastructure::{
    config::{nats::NatsConfig, postgres::PostgresConfig},
    database::postgres::connection::create_postgres_pool,
    messaging::nats::{connection::create_jetstream, producer::NatsOutboxPublisher},
    outbox::dispatcher::OutboxDispatcher,
};
use tokio::signal;

use crate::config::WorkerConfig;

mod config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let worker_config = WorkerConfig::from_env()?;
    let postgres_config = PostgresConfig::from_env()?;
    let nats_config = NatsConfig::from_env()?;

    let pool = create_postgres_pool(postgres_config).await?;
    let jetstream = create_jetstream(&nats_config).await?;

    let publisher = Arc::new(NatsOutboxPublisher::new(
        jetstream,
        nats_config.subject_prefix.clone(),
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
