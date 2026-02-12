use std::error::Error;

use actix_web::{App, HttpServer, middleware::Logger, web};
use application::usecases::room::{create_room::CreateRoom, generate_token::GenerateToken};
use infrastructure::{
    config::{livekit::LiveKitConfig, postgres::PostgresConfig},
    database::postgres::{
        connection::create_postgres_pool,
        repositories::{room::PostgresRoomRepository, user::PostgresUserRepository},
    },
    hasher::bcrypt_hasher::BcryptHasher,
    voice::livekit_service::LiveKitService,
};

use crate::{config::ServerConfig, routes, shared::state::AppState};

pub async fn run() -> Result<(), Box<dyn Error>> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let server_config = ServerConfig::from_env()?;
    let postgres_config = PostgresConfig::from_env()?;
    let livekit_config = LiveKitConfig::from_env()?;

    let postgres_pool = create_postgres_pool(postgres_config).await?;

    let room_repository = PostgresRoomRepository::new(postgres_pool.clone());
    let user_repository = PostgresUserRepository::new(postgres_pool.clone());

    let hasher = BcryptHasher::default();
    let livekit_service = LiveKitService::new(livekit_config);

    let create_room_usecase = CreateRoom::new(room_repository, hasher);
    let generate_token_usecase = GenerateToken::new(livekit_service, user_repository);

    let app_state = web::Data::new(AppState::new(
        create_room_usecase,
        generate_token_usecase,
        server_config.jwt_secret,
    ));

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(Logger::default())
            .configure(routes::configure)
    })
    .bind((server_config.host, server_config.port))?
    .run()
    .await?;

    Ok(())
}
