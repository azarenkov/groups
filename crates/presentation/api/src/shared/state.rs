use application::usecases::{
    auth::{login::LoginUser, register::RegisterUser},
    room::{
        create_room::CreateRoom, delete_room::DeleteRoom, generate_token::GenerateToken,
        publish_room_event::PublishRoomEvent,
    },
    server::{create_server::CreateServer, delete_server::DeleteServer},
};
use infrastructure::{
    database::postgres::repositories::{
        room::PostgresRoomRepository, server::PostgresServerRepository,
        user::PostgresUserRepository,
    },
    hasher::bcrypt_hasher::BcryptHasher,
    messaging::nats::producer::NatsProducer,
    voice::livekit_service::LiveKitService,
};

pub struct AppState {
    create_room_usecase: CreateRoom<PostgresRoomRepository, BcryptHasher, PostgresServerRepository>,
    generate_token_usecase:
        GenerateToken<LiveKitService, PostgresUserRepository, PostgresRoomRepository>,
    delete_room_usecase: DeleteRoom<PostgresRoomRepository>,

    register_user_usecase: RegisterUser<PostgresUserRepository, BcryptHasher>,
    login_user_usecase: LoginUser<PostgresUserRepository, BcryptHasher>,

    create_server_usecase: CreateServer<PostgresServerRepository>,
    delete_server_usecase: DeleteServer<PostgresServerRepository>,

    publish_room_event_usecase: PublishRoomEvent<NatsProducer, PostgresRoomRepository>,

    jwt_secret: String,
}

impl AppState {
    pub fn new(
        create_room_usecase: CreateRoom<
            PostgresRoomRepository,
            BcryptHasher,
            PostgresServerRepository,
        >,
        generate_token_usecase: GenerateToken<
            LiveKitService,
            PostgresUserRepository,
            PostgresRoomRepository,
        >,
        delete_room_usecase: DeleteRoom<PostgresRoomRepository>,
        register_user_usecase: RegisterUser<PostgresUserRepository, BcryptHasher>,
        login_user_usecase: LoginUser<PostgresUserRepository, BcryptHasher>,
        create_server_usecase: CreateServer<PostgresServerRepository>,
        delete_server_usecase: DeleteServer<PostgresServerRepository>,
        publish_room_event_usecase: PublishRoomEvent<NatsProducer, PostgresRoomRepository>,
        jwt_secret: String,
    ) -> Self {
        Self {
            create_room_usecase,
            generate_token_usecase,
            delete_room_usecase,
            register_user_usecase,
            login_user_usecase,
            create_server_usecase,
            delete_server_usecase,
            publish_room_event_usecase,
            jwt_secret,
        }
    }

    pub fn create_room_usecase(
        &self,
    ) -> &CreateRoom<PostgresRoomRepository, BcryptHasher, PostgresServerRepository> {
        &self.create_room_usecase
    }

    pub fn generate_token_usecase(
        &self,
    ) -> &GenerateToken<LiveKitService, PostgresUserRepository, PostgresRoomRepository> {
        &self.generate_token_usecase
    }

    pub fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }

    pub fn register_user_usecase(&self) -> &RegisterUser<PostgresUserRepository, BcryptHasher> {
        &self.register_user_usecase
    }

    pub fn login_user_usecase(&self) -> &LoginUser<PostgresUserRepository, BcryptHasher> {
        &self.login_user_usecase
    }

    pub fn create_server_usecase(&self) -> &CreateServer<PostgresServerRepository> {
        &self.create_server_usecase
    }

    pub fn delete_server_usecase(&self) -> &DeleteServer<PostgresServerRepository> {
        &self.delete_server_usecase
    }

    pub fn delete_room_usecase(&self) -> &DeleteRoom<PostgresRoomRepository> {
        &self.delete_room_usecase
    }

    pub fn publish_room_event_usecase(
        &self,
    ) -> &PublishRoomEvent<NatsProducer, PostgresRoomRepository> {
        &self.publish_room_event_usecase
    }
}
