use std::error::Error;

use actix_cors::Cors;
use actix_web::{App, HttpServer, web};
use platform::{
    config::{nats::NatsConfig, postgres::PostgresConfig},
    database::postgres::connection::{create_postgres_pool, ping_pool},
    messaging::nats::connection::{create_jetstream, is_nats_connected},
};
use api_common::health::{self, Readiness};
use tokio::signal;

mod config;
mod connections;
mod presence_recipients;
mod ws;

use crate::{
    config::WebsocketConfig,
    connections::ConnectionManager,
    presence_recipients::PresenceRecipientResolver,
    ws::{WsState, presence_preference, ws_handler},
};

pub async fn run() -> Result<(), Box<dyn Error>> {
    let cfg = WebsocketConfig::from_env()?;
    let nats_cfg = NatsConfig::from_env()?;
    let pg_cfg = PostgresConfig::from_env()?;

    let jetstream = create_jetstream(&nats_cfg).await?;
    let pg_pool = create_postgres_pool(pg_cfg).await?;
    let presence_resolver = PresenceRecipientResolver::new(pg_pool.clone());

    let core = jetstream.client().clone();
    let prefix = nats_cfg.subject_prefix.trim_end_matches('.');
    let user_subject_prefix = format!("{}.realtime.user", prefix);
    let presence_subject = format!("{}.realtime.presence", prefix);
    let presence_preference_subject =
        format!("{}.realtime.presence.preference", prefix);
    let manager = ConnectionManager::new(
        core,
        user_subject_prefix,
        presence_subject,
        presence_preference_subject,
        presence_resolver,
    );
    manager.start_presence_subscription().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to subscribe to presence subject");
        Box::<dyn Error>::from(format!("presence subscribe failed: {e}"))
    })?;
    manager
        .start_presence_preference_subscription()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to subscribe to presence preference subject");
            Box::<dyn Error>::from(format!("presence preference subscribe failed: {e}"))
        })?;

    let readiness = {
        let jetstream = jetstream.clone();
        let pool = pg_pool.clone();
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

    let health_server = health::bind(cfg.websocket_health_port, readiness.clone())?;
    let health_handle = actix_web::rt::spawn(async move {
        if let Err(e) = health_server.await {
            tracing::error!(error = %e, "Health server stopped");
        }
    });

    let ws_state = web::Data::new(WsState {
        manager: manager.clone(),
        jwt_secret: cfg.jwt_secret.clone(),
        outbound_buffer: cfg.websocket_outbound_buffer,
    });

    let bind_address = cfg.bind_address.clone();
    let server = HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);
        App::new()
            .wrap(cors)
            .app_data(ws_state.clone())
            .service(presence_preference)
            .service(ws_handler)
    })
    .bind(&bind_address)?
    .run();
    let server_handle = server.handle();
    let server_task = actix_web::rt::spawn(server);

    tracing::info!(%bind_address, "Websocket service started");
    readiness.mark_ready();

    signal::ctrl_c().await?;
    tracing::info!("Shutdown signal received, stopping HTTP server");

    server_handle.stop(true).await;
    let _ = server_task.await;
    health_handle.abort();

    Ok(())
}
