use std::{error::Error, sync::Arc};

use actix_cors::Cors;
use actix_web::{App, HttpServer, middleware::Logger, web};
use application::usecases::{
    auth::{login::LoginUser, register::RegisterUser},
    channel::{
        create_channel::CreateChannel, delete_channel::DeleteChannel, join_channel::JoinChannel,
        leave_channel::LeaveChannel,
    },
    server::{
        create_server::CreateServer, delete_server::DeleteServer,
        list_server_channels::ListServerChannels, list_server_members::ListServerMembers,
        list_servers_by_user::ListServersByUser,
    },
};
use infrastructure::{
    config::{
        livekit::LiveKitConfig, mailer::MailerConfig, nats::NatsConfig, postgres::PostgresConfig,
    },
    database::postgres::{
        connection::create_postgres_pool,
        repositories::{
            channel::PostgresChannelRepository, server::PostgresServerRepository,
            user::PostgresUserRepository, verification_code::PostgresVerificationCodeRepository,
        },
    },
    hasher::bcrypt_hasher::BcryptHasher,
    mailer::smtp_mailer::SmtpMailer,
    messaging::nats::{connection::create_jetstream, producer::NatsProducer},
    voice::livekit_service::LiveKitService,
};
use shared::rate_limiting::{
    config::RateLimitingConfig, create_governor_config, create_rate_limiting,
};

use crate::{config::ServerConfig, routes, shared::state::AppState};

pub async fn run() -> Result<(), Box<dyn Error>> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let server_config = ServerConfig::from_env()?;
    let postgres_config = PostgresConfig::from_env()?;
    let livekit_config = LiveKitConfig::from_env()?;
    let nats_config = NatsConfig::from_env()?;
    let rate_limiting_config = RateLimitingConfig::from_env()?;
    let mailer_config = MailerConfig::from_env()?;

    let postgres_pool = create_postgres_pool(postgres_config).await?;
    let nats_jetstream = create_jetstream(&nats_config).await?;

    let channel_repository = Arc::new(PostgresChannelRepository::new(postgres_pool.clone()));
    let user_repository = Arc::new(PostgresUserRepository::new(postgres_pool.clone()));
    let server_repository = Arc::new(PostgresServerRepository::new(postgres_pool.clone()));
    let verification_code_repository = Arc::new(PostgresVerificationCodeRepository::new(
        postgres_pool.clone(),
    ));

    let mailer = Arc::new(SmtpMailer::from_config(mailer_config)?);
    let hasher = Arc::new(BcryptHasher::default());
    let livekit_service = Arc::new(LiveKitService::new(livekit_config));

    let event_producer = Arc::new(NatsProducer::new(nats_jetstream, nats_config));

    let create_channel_usecase = CreateChannel::new(
        channel_repository.clone(),
        hasher.clone(),
        server_repository.clone(),
    );
    let join_channel_usecase = JoinChannel::new(
        livekit_service.clone(),
        channel_repository.clone(),
        user_repository.clone(),
        hasher.clone(),
    );
    let leave_channel_usecase = LeaveChannel::new(channel_repository.clone());
    let delete_channel_usecase = DeleteChannel::new(channel_repository.clone());
    let register_user_usecase = RegisterUser::new(
        user_repository.clone(),
        hasher.clone(),
        mailer.clone(),
        verification_code_repository.clone(),
    );
    let login_user_usecase = LoginUser::new(user_repository.clone(), hasher.clone());
    let create_server_usecase = CreateServer::new(server_repository.clone());
    let delete_server_usecase = DeleteServer::new(server_repository.clone());
    let list_servers_by_user_usecase = ListServersByUser::new(server_repository.clone());
    let list_server_members_usecase = ListServerMembers::new(server_repository.clone());
    let list_server_channels_usecase = ListServerChannels::new(server_repository.clone());

    let app_state = web::Data::new(AppState::new(
        create_channel_usecase,
        join_channel_usecase,
        leave_channel_usecase,
        delete_channel_usecase,
        register_user_usecase,
        login_user_usecase,
        create_server_usecase,
        delete_server_usecase,
        list_servers_by_user_usecase,
        list_server_members_usecase,
        list_server_channels_usecase,
        server_config.jwt_secret,
    ));

    let governor_config = create_governor_config(rate_limiting_config)
        .ok_or("Failed to create rate limiting configuration: invalid parameters")?;

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        let rate_limiting = create_rate_limiting(&governor_config);

        App::new()
            .app_data(app_state.clone())
            .wrap(cors)
            .wrap(Logger::default())
            .wrap(rate_limiting)
            .configure(routes::configure)
    })
    .bind((server_config.host, server_config.port))?
    .run()
    .await?;

    Ok(())
}
