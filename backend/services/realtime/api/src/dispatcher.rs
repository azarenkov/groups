use std::error::Error;

use infrastructure_common::{
    config::{nats::NatsConfig, postgres::PostgresConfig},
    database::postgres::connection::{create_postgres_pool, ping_pool},
    messaging::nats::connection::{create_jetstream, is_nats_connected},
};
use api_common::health::{self, Liveness, Readiness};
use api_common::shutdown::{ShutdownConfig, ShutdownSignal};
use tokio::sync::watch;

use std::{sync::Arc, time::Duration};

use realtime_core::usecases::{apply_projection::ApplyProjection, dispatch_event::DispatchEvent};
use realtime_infrastructure::{
    database::postgres::projection::PostgresRealtimeProjection,
    media_client::relations::MessagingRelationsClient,
    messaging::nats::{
        partitions::{DispatcherSettings, run_partitioned},
        projection_catchup::{CatchupSpec, run_catchup},
        publisher::NatsRealtimePublisher,
    },
};

use crate::config::DispatcherConfig;

pub async fn run(mut shutdown: ShutdownSignal) -> Result<(), Box<dyn Error>> {
    let cfg = DispatcherConfig::from_env()?;
    let nats_cfg = NatsConfig::from_env()?;
    let pg_cfg = PostgresConfig::from_env()?;

    let jetstream = create_jetstream(&nats_cfg).await?;
    let pool = create_postgres_pool(pg_cfg).await?;

    let core = jetstream.client().clone();
    let user_subject_prefix = format!(
        "{}.realtime.user",
        nats_cfg.subject_prefix.trim_end_matches('.'),
    );
    let publisher = Arc::new(NatsRealtimePublisher::new(core, user_subject_prefix));
    let projection = Arc::new(PostgresRealtimeProjection::new(pool.clone()));
    let messaging_internal_url = std::env::var("MESSAGING_INTERNAL_URL")
        .unwrap_or_else(|_| "http://messaging-service:8080".to_string());
    let blocks = Arc::new(MessagingRelationsClient::new(&messaging_internal_url));
    let dispatch = Arc::new(DispatchEvent::new(projection.clone(), publisher, blocks.clone()));
    let apply = Arc::new(ApplyProjection::new(projection, blocks));
    let settings = DispatcherSettings {
        user_stream: cfg.dispatcher_user_stream.clone(),
        channel_stream: cfg.dispatcher_channel_stream.clone(),
        message_stream: cfg.dispatcher_message_stream.clone(),
        dm_message_stream: cfg.dispatcher_dm_message_stream.clone(),
        server_stream: cfg.dispatcher_server_stream.clone(),
        consumer_prefix: cfg.dispatcher_consumer_prefix.clone(),
        max_deliver: cfg.dispatcher_max_deliver,
    };

    let readiness = {
        let jetstream = jetstream.clone();
        let pool = pool.clone();
        Readiness::builder()
            .add_probe(move || {
                let jetstream = jetstream.clone();
                async move { is_nats_connected(&jetstream) }
            })
            .add_probe(move || {
                let pool = pool.clone();
                async move { ping_pool(&pool).await }
            })
            .build()
    };

    let stale_after = Duration::from_secs(cfg.dispatcher_liveness_stale_secs);
    let liveness = Liveness::default();
    let health_server = health::bind(cfg.dispatcher_health_port, readiness.clone(), liveness.clone())?;
    let health_server_handle = health_server.handle();
    let health_handle = actix_web::rt::spawn(async move {
        if let Err(e) = health_server.await {
            tracing::error!(error = %e, "Health server stopped");
        }
    });

    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    remove_legacy_consumers(&jetstream, &cfg).await;

    let mut consumer_handles = Vec::new();
    {
        let jetstream = jetstream.clone();
        let settings = settings.clone();
        let prefix = nats_cfg.subject_prefix.clone();
        let partitions = nats_cfg.event_partitions;
        let replicas = nats_cfg.replicas;
        let dispatch = dispatch.clone();
        let rx = shutdown_rx.clone();
        let liveness = liveness.clone();
        consumer_handles.push(actix_web::rt::spawn(async move {
            run_partitioned(
                jetstream, settings, prefix, partitions, replicas, dispatch, rx, liveness, stale_after,
            )
            .await;
        }));
    }

    let prefix = nats_cfg.subject_prefix.trim_end_matches('.').to_string();
    let catchup_specs = vec![
        CatchupSpec {
            stream: cfg.dispatcher_user_stream.clone(),
            consumer_name: format!("{}-projection-user", cfg.dispatcher_consumer_prefix),
            filter_subject: format!("{prefix}.user.>"),
            dlq_subject: format!("{prefix}.dlq.realtime.projection_user"),
        },
        CatchupSpec {
            stream: cfg.dispatcher_server_stream.clone(),
            consumer_name: format!("{}-projection-server", cfg.dispatcher_consumer_prefix),
            filter_subject: format!("{prefix}.server.>"),
            dlq_subject: format!("{prefix}.dlq.realtime.projection_server"),
        },
        CatchupSpec {
            stream: cfg.dispatcher_dm_message_stream.clone(),
            consumer_name: format!("{}-projection-dm", cfg.dispatcher_consumer_prefix),
            filter_subject: format!("{prefix}.dm_message.>"),
            dlq_subject: format!("{prefix}.dlq.realtime.projection_dm"),
        },
    ];
    for spec in catchup_specs {
        let jetstream = jetstream.clone();
        let apply = apply.clone();
        let rx = shutdown_rx.clone();
        let heartbeat = liveness.register(stale_after);
        let liveness = liveness.clone();
        consumer_handles.push(actix_web::rt::spawn(async move {
            run_catchup(jetstream, spec, apply, rx, heartbeat.clone()).await;
            liveness.release(&heartbeat);
        }));
    }

    tracing::info!("Event dispatcher started");
    readiness.mark_ready();

    shutdown.wait().await;
    readiness.mark_not_ready();
    let _ = shutdown_tx.send(true);
    tracing::info!("Stopping consumers");
    let drain_delay = tokio::time::sleep(ShutdownConfig::from_env().drain_delay);
    let consumers_done = async {
        for h in consumer_handles {
            let _ = h.await;
        }
    };
    tokio::join!(consumers_done, drain_delay);
    health_server_handle.stop(false).await;
    let _ = health_handle.await;

    Ok(())
}

fn legacy_consumers(cfg: &DispatcherConfig) -> Vec<(String, String)> {
    let consumer_prefix = &cfg.dispatcher_consumer_prefix;
    vec![
        (
            cfg.dispatcher_user_stream.clone(),
            format!("{consumer_prefix}-user"),
        ),
        (
            cfg.dispatcher_channel_stream.clone(),
            format!("{consumer_prefix}-channel"),
        ),
        (
            cfg.dispatcher_message_stream.clone(),
            format!("{consumer_prefix}-message"),
        ),
        (
            cfg.dispatcher_dm_message_stream.clone(),
            format!("{consumer_prefix}-dm-message"),
        ),
        (
            cfg.dispatcher_server_stream.clone(),
            format!("{consumer_prefix}-server"),
        ),
    ]
}

async fn remove_legacy_consumers(
    jetstream: &async_nats::jetstream::Context,
    cfg: &DispatcherConfig,
) {
    for (stream_name, consumer_name) in legacy_consumers(cfg) {
        if let Ok(stream) = jetstream.get_stream(&stream_name).await
            && stream.delete_consumer(&consumer_name).await.is_ok()
        {
            tracing::info!(%stream_name, %consumer_name, "Removed legacy dispatcher consumer");
        }
    }
}

