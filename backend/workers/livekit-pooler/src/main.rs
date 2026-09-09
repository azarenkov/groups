use std::{error::Error, sync::Arc, time::Duration};

use api_common::health::{self, Heartbeat, Liveness, Readiness};
use api_common::shutdown::wait_for_signal;
use tokio::sync::watch;
use infrastructure_common::config::{livekit::LiveKitConfig, postgres::PostgresConfig};
use messaging::usecases::channel::{
    abort_voice_connection::AbortVoiceConnection, reconcile_voice_presence::ReconcileVoicePresence,
    track_media_connection::TrackMediaConnection,
};
use messaging_infrastructure::{
    database::postgres::{
        connection::{create_postgres_pool, ping_pool},
        repositories::{channel::PostgresChannelRepository, channel_unit_of_work::PostgresChannelUnitOfWork},
    },
    voice::livekit_room_inspector::LiveKitRoomInspector,
};
use crate::config::PoolerConfig;

mod config;

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
    let heartbeat = Heartbeat::new(Duration::from_secs(
        pooler_config.livekit_pooler_liveness_stale_secs,
    ));
    let health_server = health::bind(
        pooler_config.livekit_pooler_health_port,
        readiness.clone(),
        Liveness::default().watch(heartbeat.clone()),
    )?;
    let health_server_handle = health_server.handle();
    let health_handle = actix_web::rt::spawn(async move {
        if let Err(e) = health_server.await {
            tracing::error!(error = %e, "Health server stopped");
        }
    });

    let channel_repository = Arc::new(PostgresChannelRepository::new(pool.clone()));
    let channel_unit_of_work = Arc::new(PostgresChannelUnitOfWork::new(pool));
    let inspector = Arc::new(LiveKitRoomInspector::new(
        &livekit_config,
        Duration::from_secs(pooler_config.livekit_pooler_request_timeout_secs),
    ));
    let reconcile = ReconcileVoicePresence::new(
        channel_repository.clone(),
        inspector,
        Arc::new(AbortVoiceConnection::new(channel_unit_of_work.clone())),
        Arc::new(TrackMediaConnection::new(channel_unit_of_work)),
        Duration::from_secs(pooler_config.livekit_pooler_grace_secs),
        pooler_config.livekit_pooler_max_aborts_per_pass,
    );

    let interval = Duration::from_secs(pooler_config.livekit_pooler_interval_secs);

    tracing::info!(
        interval_secs = pooler_config.livekit_pooler_interval_secs,
        grace_secs = pooler_config.livekit_pooler_grace_secs,
        request_timeout_secs = pooler_config.livekit_pooler_request_timeout_secs,
        max_aborts_per_pass = pooler_config.livekit_pooler_max_aborts_per_pass,
        "LiveKit pooler started",
    );

    readiness.mark_ready();

    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    actix_web::rt::spawn({
        let readiness = readiness.clone();
        async move {
            wait_for_signal().await;
            readiness.mark_not_ready();
            let _ = shutdown_tx.send(true);
        }
    });

    loop {
        heartbeat.beat();
        if *shutdown_rx.borrow() {
            tracing::info!("Shutdown requested, exiting");
            break;
        }
        let checkpoint = || {
            heartbeat.beat();
            !*shutdown_rx.borrow()
        };
        match reconcile.execute_with_checkpoint(&checkpoint).await {
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
            _ = shutdown_rx.changed() => {
                tracing::info!("Shutdown requested, exiting");
                break;
            }
            _ = heartbeat.keep_alive_for(interval) => {}
        }
    }

    health_server_handle.stop(false).await;
    let _ = health_handle.await;
    Ok(())
}
