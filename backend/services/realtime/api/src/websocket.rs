use std::error::Error;

use actix_web::{App, HttpServer, web};
use infrastructure_common::{
    config::{nats::NatsConfig, postgres::PostgresConfig},
    database::postgres::connection::{create_postgres_pool, ping_pool},
    messaging::nats::connection::{create_jetstream, is_nats_connected},
};
use realtime_infrastructure::database::postgres::{MIGRATOR, SCHEMA};
use api_common::health::{self, Liveness, Readiness};
use api_common::shutdown::{ShutdownConfig, ShutdownSignal, drain};

use std::sync::Arc;

use realtime_core::usecases::block_aware_directory::BlockAwareDirectory;
use realtime_infrastructure::{connections::ConnectionManager, database::postgres::projection::PostgresRealtimeProjection, media_client::relations::MessagingRelationsClient};

use crate::{
    config::WebsocketConfig,
    ws::{WsState, presence_snapshot, ws_handler},
};

pub async fn run(mut shutdown: ShutdownSignal) -> Result<(), Box<dyn Error>> {
    let cfg = WebsocketConfig::from_env()?;
    let nats_cfg = NatsConfig::from_env()?;
    let pg_cfg = PostgresConfig::from_env_for(SCHEMA)?;

    let jetstream = create_jetstream(&nats_cfg).await?;
    let pg_pool = create_postgres_pool(pg_cfg, SCHEMA, Some(&MIGRATOR)).await?;
    let messaging_internal_url = api_common::internal_client::InternalService::Messaging.base_url();
    let presence_resolver = Arc::new(BlockAwareDirectory::new(
        Arc::new(PostgresRealtimeProjection::new(pg_pool.clone())),
        Arc::new(MessagingRelationsClient::new(&messaging_internal_url)),
    ));

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
    manager.start_preference_reconciliation();

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

    let health_server =
        health::bind(cfg.websocket_health_port, readiness.clone(), Liveness::default())?;
    let health_server_handle = health_server.handle();
    let health_handle = actix_web::rt::spawn(async move {
        if let Err(e) = health_server.await {
            tracing::error!(error = %e, "Health server stopped");
        }
    });

    let ws_state = web::Data::new(WsState {
        manager: manager.clone(),
        jwt_secret: cfg.jwt_secret.clone(),
        outbound_buffer: cfg.websocket_outbound_buffer,
        access_revocations: api_common::middleware::revocation::AccessRevocations::from_env().await,
    });

    let bind_address = cfg.bind_address.clone();
    let shutdown_config = ShutdownConfig::from_env();
    let server = HttpServer::new(move || {
        let cors = api_common::cors::cors();
        App::new()
            .wrap(cors)
            .app_data(ws_state.clone())
            .service(presence_snapshot)
            .service(ws_handler)
    })
    .disable_signals()
    .shutdown_timeout(shutdown_config.shutdown_timeout.as_secs())
    .bind(&bind_address)?
    .run();
    let server_handle = server.handle();
    let server_task = actix_web::rt::spawn(server);
    tokio::pin!(server_task);

    tracing::info!(%bind_address, "Websocket service started");
    readiness.mark_ready();

    tokio::select! {
        finished = &mut server_task => {
            readiness.mark_not_ready();
            health_server_handle.stop(false).await;
            let _ = health_handle.await;
            return finished
                .map_err(Box::<dyn Error>::from)?
                .map_err(Box::<dyn Error>::from);
        }
        _ = shutdown.wait() => {}
    }
    drain(&readiness, &shutdown_config).await;
    tracing::info!("Stopping websocket server");

    server_handle.stop(true).await;
    let _ = server_task.await;
    health_server_handle.stop(false).await;
    let _ = health_handle.await;

    Ok(())
}
