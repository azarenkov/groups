use std::{error::Error, sync::Arc};

use actix_cors::Cors;
use actix_web::{App, HttpServer, middleware::Logger, web};
use application::usecases::{
    auth::{
        confirm_registration::ConfirmRegistration,
        issue_verification_code::IssueVerificationCode, login::LoginUser, logout::Logout,
        refresh_tokens::RefreshTokens, register::RegisterUser,
    },
    channel::{
        abort_voice_connection::AbortVoiceConnection, create_channel::CreateChannel,
        delete_channel::DeleteChannel, join_voice_channel::JoinVoiceChannel,
        join_voice_connection::JoinVoiceConnection,
        leave_voice_connection::LeaveVoiceConnection, mute_channel::MuteChannel,
        unmute_channel::UnmuteChannel,
    },
    server::{
        create_server::CreateServer, delete_server::DeleteServer,
        list_server_channels::ListServerChannels, list_server_members::ListServerMembers,
        list_servers_by_user::ListServersByUser,
    },
    webhook::voice::HandleVoiceWebhook,
};
use infrastructure::{
    auth::jwt_token_issuer::JwtTokenIssuer,
    config::{livekit::LiveKitConfig, postgres::PostgresConfig},
    database::postgres::{
        connection::{create_postgres_pool, ping_pool},
        repositories::{
            channel::PostgresChannelRepository, refresh_token::PostgresRefreshTokenRepository,
            server::PostgresServerRepository, user::PostgresUserRepository,
            verification_code::PostgresVerificationCodeRepository,
        },
    },
    hasher::bcrypt_hasher::BcryptHasher,
    voice::{
        livekit_service::{LiveKitService, is_livekit_reachable},
        livekit_webhook_receiver::LiveKitWebhookReceiver,
    },
};
use shared::{
    health::Readiness,
    rate_limiting::{config::RateLimitingConfig, create_rate_limiter},
};

use crate::{config::ServerConfig, routes, shared::state::AppState};

pub async fn run() -> Result<(), Box<dyn Error>> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let server_config = ServerConfig::from_env()?;
    let postgres_config = PostgresConfig::from_env()?;
    let livekit_config = LiveKitConfig::from_env()?;
    let rate_limiting_config = RateLimitingConfig::from_env()?;

    let postgres_pool = create_postgres_pool(postgres_config).await?;
    let livekit_url = livekit_config.url().to_string();

    let channel_repository = Arc::new(PostgresChannelRepository::new(postgres_pool.clone()));
    let user_repository = Arc::new(PostgresUserRepository::new(postgres_pool.clone()));
    let server_repository = Arc::new(PostgresServerRepository::new(postgres_pool.clone()));
    let verification_code_repository = Arc::new(PostgresVerificationCodeRepository::new(
        postgres_pool.clone(),
    ));
    let refresh_token_repository =
        Arc::new(PostgresRefreshTokenRepository::new(postgres_pool.clone()));

    let hasher = Arc::new(BcryptHasher::default());
    let livekit_webhook_receiver = Arc::new(LiveKitWebhookReceiver::new(&livekit_config));
    let livekit_service = Arc::new(LiveKitService::new(livekit_config));
    let token_issuer = Arc::new(JwtTokenIssuer::new(server_config.jwt_secret.clone()));

    let create_channel_usecase = CreateChannel::new(
        channel_repository.clone(),
        hasher.clone(),
        server_repository.clone(),
    );
    let join_voice_channel_usecase = JoinVoiceChannel::new(
        livekit_service.clone(),
        channel_repository.clone(),
        user_repository.clone(),
        hasher.clone(),
    );
    let mute_channel_usecase =
        MuteChannel::new(channel_repository.clone(), user_repository.clone());
    let unmute_channel_usecase =
        UnmuteChannel::new(channel_repository.clone(), user_repository.clone());
    let delete_channel_usecase = DeleteChannel::new(channel_repository.clone());
    let register_user_usecase = RegisterUser::new(
        user_repository.clone(),
        hasher.clone(),
        token_issuer.clone(),
    );
    let issue_verification_code_usecase = IssueVerificationCode::new(
        user_repository.clone(),
        verification_code_repository.clone(),
        hasher.clone(),
    );
    let login_user_usecase = LoginUser::new(
        user_repository.clone(),
        hasher.clone(),
        refresh_token_repository.clone(),
        token_issuer.clone(),
    );
    let confirm_registration_usecase = ConfirmRegistration::new(
        user_repository.clone(),
        hasher.clone(),
        verification_code_repository.clone(),
    );
    let refresh_tokens_usecase = RefreshTokens::new(
        refresh_token_repository.clone(),
        user_repository.clone(),
        token_issuer.clone(),
    );
    let logout_usecase = Logout::new(refresh_token_repository.clone(), token_issuer.clone());

    let join_voice_connection_usecase = JoinVoiceConnection::new(channel_repository.clone());
    let abort_voice_connection_usecase = AbortVoiceConnection::new(channel_repository.clone());
    let leave_voice_connection_usecase = LeaveVoiceConnection::new(channel_repository.clone());
    let handle_livekit_webhook_usecase = HandleVoiceWebhook::new(
        livekit_webhook_receiver.clone(),
        join_voice_connection_usecase,
        abort_voice_connection_usecase,
        leave_voice_connection_usecase,
    );

    let create_server_usecase = CreateServer::new(server_repository.clone());
    let delete_server_usecase = DeleteServer::new(server_repository.clone());
    let list_servers_by_user_usecase = ListServersByUser::new(server_repository.clone());
    let list_server_members_usecase = ListServerMembers::new(server_repository.clone());
    let list_server_channels_usecase = ListServerChannels::new(server_repository.clone());

    let app_state = web::Data::new(AppState::new(
        create_channel_usecase,
        join_voice_channel_usecase,
        mute_channel_usecase,
        unmute_channel_usecase,
        delete_channel_usecase,
        register_user_usecase,
        issue_verification_code_usecase,
        login_user_usecase,
        confirm_registration_usecase,
        refresh_tokens_usecase,
        logout_usecase,
        handle_livekit_webhook_usecase,
        create_server_usecase,
        delete_server_usecase,
        list_servers_by_user_usecase,
        list_server_members_usecase,
        list_server_channels_usecase,
        server_config.jwt_secret,
        server_config.trust_forwarded_headers,
    ));

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
            .max_age(3600);

        App::new()
            .app_data(app_state.clone())
            .app_data(readiness_data.clone())
            .app_data(rate_limiter_data.clone())
            .wrap(cors)
            .wrap(Logger::default())
            .configure(routes::configure)
    })
    .bind((server_config.host, server_config.port))?
    .run()
    .await?;

    Ok(())
}
