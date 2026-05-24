use std::error::Error;

use infrastructure::{
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
mod publisher;
mod recipients;

use crate::{
    config::DispatcherConfig,
    consumer::{ConsumerSpec, run_consumer},
    publisher::Publisher,
    recipients::RecipientResolver,
};

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    shared::observability::init("event-dispatcher");

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

    let health_server = health::bind(cfg.health_port, readiness.clone())?;
    let health_handle = actix_web::rt::spawn(async move {
        if let Err(e) = health_server.await {
            tracing::error!(error = %e, "Health server stopped");
        }
    });

    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let consumer_specs = build_consumer_specs(&cfg, &nats_cfg.subject_prefix);
    let mut consumer_handles = Vec::new();
    for spec in consumer_specs {
        let jetstream = jetstream.clone();
        let resolver = resolver.clone();
        let publisher = publisher.clone();
        let rx = shutdown_rx.clone();
        consumer_handles.push(actix_web::rt::spawn(async move {
            run_consumer(jetstream, spec, resolver, publisher, rx).await;
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

fn build_consumer_specs(cfg: &DispatcherConfig, prefix: &str) -> Vec<ConsumerSpec> {
    let prefix = prefix.trim_end_matches('.');
    let consumer_prefix = &cfg.dispatcher_consumer_prefix;
    let max_deliver = cfg.dispatcher_max_deliver;
    let dlq = |kind: &str| format!("{prefix}.dlq.realtime.{kind}");
    vec![
        ConsumerSpec {
            stream: cfg.dispatcher_user_stream.clone(),
            consumer_name: format!("{consumer_prefix}-user"),
            filter_subject: format!("{prefix}.user.>"),
            dlq_subject: dlq("user"),
            max_deliver,
        },
        ConsumerSpec {
            stream: cfg.dispatcher_channel_stream.clone(),
            consumer_name: format!("{consumer_prefix}-channel"),
            filter_subject: format!("{prefix}.channel.>"),
            dlq_subject: dlq("channel"),
            max_deliver,
        },
        ConsumerSpec {
            stream: cfg.dispatcher_message_stream.clone(),
            consumer_name: format!("{consumer_prefix}-message"),
            filter_subject: format!("{prefix}.message.>"),
            dlq_subject: dlq("message"),
            max_deliver,
        },
        ConsumerSpec {
            stream: cfg.dispatcher_dm_message_stream.clone(),
            consumer_name: format!("{consumer_prefix}-dm-message"),
            filter_subject: format!("{prefix}.dm_message.>"),
            dlq_subject: dlq("dm_message"),
            max_deliver,
        },
        ConsumerSpec {
            stream: cfg.dispatcher_server_stream.clone(),
            consumer_name: format!("{consumer_prefix}-server"),
            filter_subject: format!("{prefix}.server.>"),
            dlq_subject: dlq("server"),
            max_deliver,
        },
    ]
}
