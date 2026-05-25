use std::error::Error;

use actix_web::{App, HttpServer, web};
use infrastructure::{
    config::nats::NatsConfig,
    messaging::nats::connection::{create_jetstream, is_nats_connected},
};
use shared::health::{self, Readiness};
use tokio::signal;

mod config;
mod connections;
mod ws;

use crate::{
    config::WebsocketConfig,
    connections::ConnectionManager,
    ws::{WsState, ws_handler},
};

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    shared::observability::init("websocket");

    let cfg = WebsocketConfig::from_env()?;
    let nats_cfg = NatsConfig::from_env()?;

    let jetstream = create_jetstream(&nats_cfg).await?;

    let core = jetstream.client().clone();
    let prefix = nats_cfg.subject_prefix.trim_end_matches('.');
    let user_subject_prefix = format!("{}.realtime.user", prefix);
    let presence_subject = format!("{}.realtime.presence", prefix);
    let manager = ConnectionManager::new(core, user_subject_prefix, presence_subject);
    if let Err(e) = manager.start_presence_subscription().await {
        tracing::error!(error = %e, "Failed to subscribe to presence subject");
    }

    let readiness = {
        let jetstream = jetstream.clone();
        Readiness::builder()
            .add_probe(move || {
                let jetstream = jetstream.clone();
                async move { is_nats_connected(&jetstream) }
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
        App::new()
            .app_data(ws_state.clone())
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
