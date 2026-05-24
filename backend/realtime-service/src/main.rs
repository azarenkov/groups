use std::error::Error;

use actix_web::{App, HttpServer, web};
use infrastructure::{
    config::{nats::NatsConfig, postgres::PostgresConfig},
    database::postgres::connection::{create_postgres_pool, ping_pool},
    messaging::nats::connection::{create_jetstream, is_nats_connected},
};
use shared::health::{self, Readiness};
use tokio::{signal, sync::watch};

mod config;
mod connections;
mod consumer;
mod dispatch;
mod envelope;
mod recipients;
mod ws;

use crate::{
    config::RealtimeConfig,
    connections::ConnectionManager,
    consumer::{ConsumerSpec, run_consumer},
    recipients::RecipientResolver,
    ws::{WsState, ws_handler},
};

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    shared::observability::init("realtime-service");

    let cfg = RealtimeConfig::from_env()?;
    let nats_cfg = NatsConfig::from_env()?;
    let pg_cfg = PostgresConfig::from_env()?;

    let jetstream = create_jetstream(&nats_cfg).await?;
    let pool = create_postgres_pool(pg_cfg).await?;

    let manager = ConnectionManager::new();
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
        let manager = manager.clone();
        let rx = shutdown_rx.clone();
        consumer_handles.push(actix_web::rt::spawn(async move {
            run_consumer(jetstream, spec, resolver, manager, rx).await;
        }));
    }

    let ws_state = web::Data::new(WsState {
        manager: manager.clone(),
        jwt_secret: cfg.jwt_secret.clone(),
        outbound_buffer: cfg.realtime_outbound_buffer,
    });

    let bind_address = cfg.bind_address.clone();
    let manager_metrics = manager.clone();
    let server = HttpServer::new(move || {
        App::new()
            .app_data(ws_state.clone())
            .app_data(web::Data::new(manager_metrics.clone()))
            .service(ws_handler)
    })
    .bind(&bind_address)?
    .run();
    let server_handle = server.handle();
    let server_task = actix_web::rt::spawn(server);

    tracing::info!(%bind_address, "Realtime service started");
    readiness.mark_ready();

    signal::ctrl_c().await?;
    tracing::info!("Shutdown signal received, stopping consumers and HTTP server");

    let _ = shutdown_tx.send(true);
    server_handle.stop(true).await;
    let _ = server_task.await;
    for h in consumer_handles {
        let _ = h.await;
    }
    health_handle.abort();

    Ok(())
}

fn build_consumer_specs(cfg: &RealtimeConfig, prefix: &str) -> Vec<ConsumerSpec> {
    let prefix = prefix.trim_end_matches('.');
    let consumer_prefix = &cfg.realtime_consumer_prefix;
    vec![
        ConsumerSpec {
            stream: cfg.realtime_user_stream.clone(),
            consumer_name: format!("{consumer_prefix}-user"),
            filter_subject: format!("{prefix}.user.>"),
        },
        ConsumerSpec {
            stream: cfg.realtime_channel_stream.clone(),
            consumer_name: format!("{consumer_prefix}-channel"),
            filter_subject: format!("{prefix}.channel.>"),
        },
        ConsumerSpec {
            stream: cfg.realtime_message_stream.clone(),
            consumer_name: format!("{consumer_prefix}-message"),
            filter_subject: format!("{prefix}.message.>"),
        },
        ConsumerSpec {
            stream: cfg.realtime_dm_message_stream.clone(),
            consumer_name: format!("{consumer_prefix}-dm-message"),
            filter_subject: format!("{prefix}.dm_message.>"),
        },
        ConsumerSpec {
            stream: cfg.realtime_server_stream.clone(),
            consumer_name: format!("{consumer_prefix}-server"),
            filter_subject: format!("{prefix}.server.>"),
        },
    ]
}
