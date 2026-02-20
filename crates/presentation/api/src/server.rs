use std::{error::Error, sync::Arc};

use actix_cors::Cors;
use actix_web::{App, HttpServer, middleware::Logger, web};
use application::usecases::{
    auth::{login::LoginUser, register::RegisterUser},
    room::{
        create_room::CreateRoom, delete_room::DeleteRoom, join_room::JoinRoom,
        publish_room_event::PublishRoomEvent,
    },
    server::{
        create_server::CreateServer, delete_server::DeleteServer,
        list_servers_by_user::ListServersByUser,
    },
};
use infrastructure::{
    config::{livekit::LiveKitConfig, nats::NatsConfig, postgres::PostgresConfig},
    database::postgres::{
        connection::create_postgres_pool,
        repositories::{
            room::PostgresRoomRepository, server::PostgresServerRepository,
            user::PostgresUserRepository,
        },
    },
    hasher::bcrypt_hasher::BcryptHasher,
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

    let postgres_pool = create_postgres_pool(postgres_config).await?;
    let nats_jetstream = create_jetstream(&nats_config).await?;

    let room_repository = Arc::new(PostgresRoomRepository::new(postgres_pool.clone()));
    let user_repository = Arc::new(PostgresUserRepository::new(postgres_pool.clone()));
    let server_repository = Arc::new(PostgresServerRepository::new(postgres_pool.clone()));

    let hasher = Arc::new(BcryptHasher::default());
    let livekit_service = Arc::new(LiveKitService::new(livekit_config));

    let event_producer = Arc::new(NatsProducer::new(nats_jetstream, nats_config));

    let create_room_usecase = CreateRoom::new(
        room_repository.clone(),
        hasher.clone(),
        server_repository.clone(),
    );
    let join_room_usecase = JoinRoom::new(
        livekit_service.clone(),
        room_repository.clone(),
        user_repository.clone(),
    );
    let delete_room_usecase = DeleteRoom::new(room_repository.clone());
    let register_user_usecase = RegisterUser::new(user_repository.clone(), hasher.clone());
    let login_user_usecase = LoginUser::new(user_repository.clone(), hasher.clone());
    let create_server_usecase = CreateServer::new(server_repository.clone());
    let delete_server_usecase = DeleteServer::new(server_repository.clone());
    let list_servers_by_user = ListServersByUser::new(server_repository.clone());
    let publish_room_event_usecase = PublishRoomEvent::new(event_producer.clone());

    let app_state = web::Data::new(AppState::new(
        create_room_usecase,
        join_room_usecase,
        delete_room_usecase,
        register_user_usecase,
        login_user_usecase,
        create_server_usecase,
        delete_server_usecase,
        list_servers_by_user,
        publish_room_event_usecase,
        server_config.jwt_secret,
    ));

    let governor_config = create_governor_config(rate_limiting_config)
        .ok_or_else(|| "Failed to create rate limiting configuration: invalid parameters")?;

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
