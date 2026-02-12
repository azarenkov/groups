use application::usecases::room::{create_room::CreateRoom, generate_token::GenerateToken};
use infrastructure::{
    database::postgres::repositories::{
        room::PostgresRoomRepository, user::PostgresUserRepository,
    },
    hasher::bcrypt_hasher::BcryptHasher,
    voice::livekit_service::LiveKitService,
};

pub struct AppState {
    create_room_usecase: CreateRoom<PostgresRoomRepository, BcryptHasher>,
    generate_token_usecase: GenerateToken<LiveKitService, PostgresUserRepository>,
    jwt_secret: String,
}

impl AppState {
    pub fn new(
        create_room_usecase: CreateRoom<PostgresRoomRepository, BcryptHasher>,
        generate_token_usecase: GenerateToken<LiveKitService, PostgresUserRepository>,
        jwt_secret: String,
    ) -> Self {
        Self {
            create_room_usecase,
            generate_token_usecase,
            jwt_secret,
        }
    }

    pub fn create_room_usecase(&self) -> &CreateRoom<PostgresRoomRepository, BcryptHasher> {
        &self.create_room_usecase
    }

    pub fn generate_token_usecase(&self) -> &GenerateToken<LiveKitService, PostgresUserRepository> {
        &self.generate_token_usecase
    }

    pub fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }
}
