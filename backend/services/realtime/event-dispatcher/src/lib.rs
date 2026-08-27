use std::error::Error;

use platform::{
    config::{nats::NatsConfig, postgres::PostgresConfig},
    database::postgres::connection::{create_postgres_pool, ping_pool},
    messaging::nats::connection::{create_jetstream, is_nats_connected},
};
use api_common::health::{self, Readiness};
use tokio::{signal, sync::watch};

mod config;
mod consumer;
mod dispatch;
mod envelope;
mod partitions;
mod projection_catchup;
mod publisher;
mod recipients;

use crate::{
    config::DispatcherConfig,
    partitions::run_partitioned,
    projection_catchup::{CatchupSpec, run_catchup},
    publisher::Publisher,
    recipients::RecipientResolver,
};

pub async fn run() -> Result<(), Box<dyn Error>> {
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
    let publisher = Publisher::new(core, user_subject_prefix);
    let resolver = RecipientResolver::new(pool.clone());

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

    let health_server = health::bind(cfg.dispatcher_health_port, readiness.clone())?;
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
        let cfg_clone = cfg.clone();
        let prefix = nats_cfg.subject_prefix.clone();
        let partitions = nats_cfg.event_partitions;
        let replicas = nats_cfg.replicas;
        let resolver = resolver.clone();
        let publisher = publisher.clone();
        let rx = shutdown_rx.clone();
        consumer_handles.push(actix_web::rt::spawn(async move {
            run_partitioned(
                jetstream, cfg_clone, prefix, partitions, replicas, resolver, publisher, rx,
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
    ];
    for spec in catchup_specs {
        let jetstream = jetstream.clone();
        let resolver = resolver.clone();
        let rx = shutdown_rx.clone();
        consumer_handles.push(actix_web::rt::spawn(async move {
            run_catchup(jetstream, spec, resolver, rx).await;
        }));
    }

    tracing::info!("Event dispatcher started");
    readiness.mark_ready();

    signal::ctrl_c().await?;
    tracing::info!("Shutdown signal received, stopping consumers");

    let _ = shutdown_tx.send(true);
    for h in consumer_handles {
        let _ = h.await;
    }
    health_handle.abort();

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

