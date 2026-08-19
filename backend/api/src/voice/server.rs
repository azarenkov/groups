use std::{error::Error, sync::Arc};

use actix_cors::Cors;
use actix_web::{App, HttpServer, middleware::Logger, web};
use application::usecases::{
    call::{
        accept_call::AcceptCall, add_participants::AddCallParticipants,
        decline_call::DeclineCall, get_active_call::GetActiveCall,
        get_active_call_with::GetActiveCallWith, join_call::JoinCall, leave_call::LeaveCall,
        log_call::LogCall, start_call::StartCall,
    },
    channel::{
        abort_voice_connection::AbortVoiceConnection,
        join_voice_connection::JoinVoiceConnection,
        leave_voice_connection::LeaveVoiceConnection,
        track_media_connection::TrackMediaConnection,
    },
    webhook::voice::HandleVoiceWebhook,
};
use infrastructure::{
    config::{livekit::LiveKitConfig, postgres::PostgresConfig},
    database::postgres::{
        connection::{create_postgres_pool, ping_pool},
        repositories::{
            call::PostgresCallRepository, channel::PostgresChannelRepository,
            dm_message::PostgresDmMessageRepository,
            friendship::PostgresFriendshipRepository, user::PostgresUserRepository,
        },
    },
    voice::{
        livekit_service::{LiveKitService, is_livekit_reachable},
        livekit_webhook_receiver::LiveKitWebhookReceiver,
    },
};
use shared::{
    health::Readiness,
    middleware::request_id::RequestIdMiddleware,
    rate_limiting::{config::RateLimitingConfig, create_rate_limiter},
};

use crate::{config::ServerConfig, voice::routes, voice::state::AppState};

pub async fn run() -> Result<(), Box<dyn Error>> {
    shared::observability::init("voice-service");

    let server_config = ServerConfig::from_env()?;
    let postgres_config = PostgresConfig::from_env()?;
    let livekit_config = LiveKitConfig::from_env()?;
    let rate_limiting_config = RateLimitingConfig::from_env()?;

    let postgres_pool = create_postgres_pool(postgres_config).await?;
    let livekit_url = livekit_config.url().to_string();

    let call_repository = Arc::new(PostgresCallRepository::new(postgres_pool.clone()));
    let channel_repository = Arc::new(PostgresChannelRepository::new(postgres_pool.clone()));
    let user_repository = Arc::new(PostgresUserRepository::new(postgres_pool.clone()));
    let friendship_repository =
        Arc::new(PostgresFriendshipRepository::new(postgres_pool.clone()));
    let dm_repository = Arc::new(PostgresDmMessageRepository::new(postgres_pool.clone()));

    let livekit_webhook_receiver = Arc::new(LiveKitWebhookReceiver::new(&livekit_config));
    let livekit_service = Arc::new(LiveKitService::new(livekit_config));

    let join_voice_connection_usecase = JoinVoiceConnection::new(channel_repository.clone());
    let abort_voice_connection_usecase = AbortVoiceConnection::new(channel_repository.clone());
    let leave_voice_connection_usecase = LeaveVoiceConnection::new(channel_repository.clone());
    let track_media_connection_usecase = TrackMediaConnection::new(channel_repository.clone());

    let app_state = web::Data::new(AppState {
        start_call_usecase: StartCall::new(
            call_repository.clone(),
            livekit_service.clone(),
            friendship_repository.clone(),
            user_repository.clone(),
        ),
        accept_call_usecase: AcceptCall::new(
            call_repository.clone(),
            livekit_service.clone(),
            user_repository.clone(),
        ),
        decline_call_usecase: DeclineCall::new(call_repository.clone()),
        leave_call_usecase: LeaveCall::new(call_repository.clone()),
        add_call_participants_usecase: AddCallParticipants::new(
            call_repository.clone(),
            friendship_repository.clone(),
        ),
        get_active_call_usecase: GetActiveCall::new(
            call_repository.clone(),
            livekit_service.clone(),
            user_repository.clone(),
        ),
        get_active_call_with_usecase: GetActiveCallWith::new(call_repository.clone()),
        join_call_usecase: JoinCall::new(
            call_repository.clone(),
            livekit_service.clone(),
            user_repository.clone(),
        ),
        log_call_usecase: LogCall::new(dm_repository.clone(), friendship_repository.clone()),
        handle_livekit_webhook_usecase: HandleVoiceWebhook::new(
            livekit_webhook_receiver.clone(),
            join_voice_connection_usecase,
            abort_voice_connection_usecase,
            leave_voice_connection_usecase,
            track_media_connection_usecase,
            call_repository.clone(),
        ),
    });

    let rate_limiter = create_rate_limiter(rate_limiting_config)
        .ok_or("Failed to create rate limiter: invalid parameters")?;
    let rate_limiter_data = web::Data::new(rate_limiter);

    let readiness = Readiness::builder()
        .add_probe({
            let pool = postgres_pool.clone();
            move || {
                let pool = pool.clone();
                async move { ping_pool(&pool).await }
            }
        })
        .add_probe({
            let url = livekit_url.clone();
            move || {
                let url = url.clone();
                async move { is_livekit_reachable(&url).await }
            }
        })
        .build();
    readiness.mark_ready();
    let readiness_data = web::Data::new(readiness);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .expose_headers(["x-request-id"])
            .max_age(3600);

        App::new()
            .app_data(app_state.clone())
            .app_data(readiness_data.clone())
            .app_data(rate_limiter_data.clone())
            .wrap(RequestIdMiddleware)
            .wrap(Logger::default())
            .wrap(cors)
            .configure(routes::configure)
    })
    .bind((server_config.host, server_config.port))?
    .run()
    .await?;

    Ok(())
}
