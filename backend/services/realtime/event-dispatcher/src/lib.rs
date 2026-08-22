use std::error::Error;

use platform::{
    config::{nats::NatsConfig, postgres::PostgresConfig},
    database::postgres::connection::{create_postgres_pool, ping_pool},
    messaging::nats::connection::{create_jetstream, is_nats_connected},
};
use shared::health::{self, Readiness};
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

    drain_and_remove_legacy(&jetstream, &cfg, &nats_cfg.subject_prefix, &resolver, &publisher)
        .await;

    let mut consumer_handles = Vec::new();
    {
        let jetstream = jetstream.clone();
        let cfg_clone = cfg.clone();
        let prefix = nats_cfg.subject_prefix.clone();
        let partitions = nats_cfg.event_partitions;
        let resolver = resolver.clone();
        let publisher = publisher.clone();
        let rx = shutdown_rx.clone();
        consumer_handles.push(actix_web::rt::spawn(async move {
            run_partitioned(
                jetstream, cfg_clone, prefix, partitions, resolver, publisher, rx,
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

fn legacy_consumers(cfg: &DispatcherConfig) -> Vec<(String, String, String)> {
    let consumer_prefix = &cfg.dispatcher_consumer_prefix;
    vec![
        (
            cfg.dispatcher_user_stream.clone(),
            format!("{consumer_prefix}-user"),
            "user".to_string(),
        ),
        (
            cfg.dispatcher_channel_stream.clone(),
            format!("{consumer_prefix}-channel"),
            "channel".to_string(),
        ),
        (
            cfg.dispatcher_message_stream.clone(),
            format!("{consumer_prefix}-message"),
            "message".to_string(),
        ),
        (
            cfg.dispatcher_dm_message_stream.clone(),
            format!("{consumer_prefix}-dm-message"),
            "dm_message".to_string(),
        ),
        (
            cfg.dispatcher_server_stream.clone(),
            format!("{consumer_prefix}-server"),
            "server".to_string(),
        ),
    ]
}

pub(crate) async fn drain_and_remove_legacy(
    jetstream: &async_nats::jetstream::Context,
    cfg: &DispatcherConfig,
    subject_prefix: &str,
    resolver: &RecipientResolver,
    publisher: &Publisher,
) {
    let prefix = subject_prefix.trim_end_matches('.');
    for (stream_name, consumer_name, domain) in legacy_consumers(cfg) {
        let Ok(stream) = jetstream.get_stream(&stream_name).await else {
            continue;
        };
        let Ok(info) = stream.consumer_info(&consumer_name).await else {
            continue;
        };
        if info.num_pending > 0 || info.num_ack_pending > 0 {
            let mut stream = stream.clone();
            let end_sequence = match stream.info().await {
                Ok(stream_info) => stream_info.state.last_sequence,
                Err(e) => {
                    tracing::warn!(error = %e, %stream_name, "Stream info unavailable; skipping legacy drain");
                    continue;
                }
            };
            let drained = drain_legacy_backlog(
                &stream,
                &info,
                end_sequence,
                prefix,
                &domain,
                resolver,
                publisher,
            )
            .await;
            if !drained {
                tracing::warn!(%stream_name, %consumer_name, "Legacy drain incomplete; keeping consumer for retry");
                continue;
            }
        }
        if stream.delete_consumer(&consumer_name).await.is_ok() {
            tracing::info!(%stream_name, %consumer_name, "Removed legacy dispatcher consumer");
        }
    }
}

async fn drain_legacy_backlog(
    stream: &async_nats::jetstream::stream::Stream,
    info: &async_nats::jetstream::consumer::Info,
    end_sequence: u64,
    prefix: &str,
    domain: &str,
    resolver: &RecipientResolver,
    publisher: &Publisher,
) -> bool {
    use async_nats::jetstream::consumer::{AckPolicy, DeliverPolicy, pull};
    use futures::StreamExt;

    let start_sequence = info.ack_floor.stream_sequence + 1;
    let filter = format!("{prefix}.{domain}.>");
    let drain_consumer = match stream
        .create_consumer(pull::Config {
            deliver_policy: DeliverPolicy::ByStartSequence { start_sequence },
            filter_subject: filter,
            ack_policy: AckPolicy::Explicit,
            inactive_threshold: std::time::Duration::from_secs(30),
            ..Default::default()
        })
        .await
    {
        Ok(consumer) => consumer,
        Err(e) => {
            tracing::warn!(error = %e, domain, "Failed to create legacy drain consumer");
            return false;
        }
    };

    let mut drained: u64 = 0;
    loop {
        let batch = drain_consumer
            .fetch()
            .max_messages(200)
            .expires(std::time::Duration::from_secs(2))
            .messages()
            .await;
        let mut batch = match batch {
            Ok(batch) => batch,
            Err(e) => {
                tracing::warn!(error = %e, domain, "Legacy drain fetch failed");
                return false;
            }
        };
        let mut received = 0usize;
        let mut reached_end = false;
        while let Some(message) = batch.next().await {
            let Ok(message) = message else {
                return false;
            };
            received += 1;
            let sequence = message.info().map(|i| i.stream_sequence).unwrap_or(0);
            if sequence > end_sequence {
                reached_end = true;
                break;
            }
            let subject = message.subject.to_string();
            let partitioned = subject
                .split('.')
                .any(platform::messaging::partitions::is_partition_token);
            if !partitioned {
                if let Err(e) = dispatch::dispatch(&subject, &message.payload, resolver, publisher).await
                {
                    tracing::warn!(error = %e, %subject, "Legacy drain dispatch failed; keeping backlog for retry");
                    return false;
                }
                drained += 1;
            }
            let _ = message.ack().await;
            if sequence == end_sequence {
                reached_end = true;
                break;
            }
        }
        if reached_end || received == 0 {
            break;
        }
    }
    if drained > 0 {
        tracing::info!(domain, drained, "Drained legacy dispatcher backlog");
    }
    true
}

