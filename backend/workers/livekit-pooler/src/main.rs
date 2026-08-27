use std::{error::Error, sync::Arc, time::Duration};

use messaging::usecases::channel::abort_voice_connection::AbortVoiceConnection;
use messaging::usecases::channel::track_media_connection::TrackMediaConnection;
use infrastructure_common::{
    config::{livekit::LiveKitConfig, postgres::PostgresConfig},
};
use messaging_infrastructure::{
    database::postgres::{
        connection::{create_postgres_pool, ping_pool},
        repositories::channel::PostgresChannelRepository,
    },
};
use api_common::health::{self, Readiness};
use tokio::signal;

use crate::{config::PoolerConfig, reconciler::Reconciler};

mod config;
mod reconciler;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    infrastructure_common::observability::init("livekit-pooler");

    let pooler_config = PoolerConfig::from_env()?;
    let postgres_config = PostgresConfig::from_env()?;
    let livekit_config = LiveKitConfig::from_env()?;

    let pool = create_postgres_pool(postgres_config).await?;

    let readiness = Readiness::builder()
        .add_probe({
            let pool = pool.clone();
            move || {
                let pool = pool.clone();
                async move { ping_pool(&pool).await }
            }
        })
        .build();
    let health_server =
        health::bind(pooler_config.livekit_pooler_health_port, readiness.clone())?;
    let health_handle = actix_web::rt::spawn(async move {
        if let Err(e) = health_server.await {
            tracing::error!(error = %e, "Health server stopped");
        }
    });

    let channel_repository = Arc::new(PostgresChannelRepository::new(pool.clone()));
    let abort_use_case = Arc::new(AbortVoiceConnection::new(channel_repository.clone()));
    let track_use_case = Arc::new(TrackMediaConnection::new(channel_repository));

    let reconciler = Arc::new(Reconciler::new(
        pool,
        livekit_config.url().to_string(),
        livekit_config.api_key().to_string(),
        livekit_config.api_secret().to_string(),
        abort_use_case,
        track_use_case,
        Duration::from_secs(pooler_config.livekit_pooler_grace_secs),
        Duration::from_secs(pooler_config.livekit_pooler_request_timeout_secs),
        pooler_config.livekit_pooler_max_aborts_per_pass,
    ));

    let interval = Duration::from_secs(pooler_config.livekit_pooler_interval_secs);

    tracing::info!(
        interval_secs = pooler_config.livekit_pooler_interval_secs,
        grace_secs = pooler_config.livekit_pooler_grace_secs,
        request_timeout_secs = pooler_config.livekit_pooler_request_timeout_secs,
        max_aborts_per_pass = pooler_config.livekit_pooler_max_aborts_per_pass,
        "LiveKit pooler started",
    );

    readiness.mark_ready();

    let shutdown = signal::ctrl_c();
    tokio::pin!(shutdown);

    loop {
        match reconciler.run_once().await {
            Ok(summary) => {
                if summary.db_entries_checked > 0
                    || summary.aborted > 0
                    || summary.tracks_reconciled > 0
                {
                    tracing::info!(
                        rooms_checked = summary.rooms_checked,
                        rooms_missing = summary.rooms_missing,
                        livekit_errors = summary.livekit_errors,
                        db_entries_checked = summary.db_entries_checked,
                        stale_detected = summary.stale_detected,
                        aborted = summary.aborted,
                        abort_skipped = summary.abort_skipped,
                        abort_failed = summary.abort_failed,
                        tracks_reconciled = summary.tracks_reconciled,
                        "LiveKit pooler pass completed",
                    );
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "LiveKit pooler pass failed");
            }
        }

        tokio::select! {
            _ = &mut shutdown => {
                tracing::info!("Shutdown signal received, exiting");
                break;
            }
            _ = tokio::time::sleep(interval) => {}
        }
    }

    health_handle.abort();
    Ok(())
}
