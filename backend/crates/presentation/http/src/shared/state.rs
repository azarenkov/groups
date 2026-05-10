use application::usecases::{
    auth::{login::LoginUser, register::RegisterUser},
    channel::{
        create_channel::CreateChannel, delete_channel::DeleteChannel, join_channel::JoinChannel,
    },
    server::{
        create_server::CreateServer, delete_server::DeleteServer,
        list_server_channels::ListServerChannels, list_server_members::ListServerMembers,
        list_servers_by_user::ListServersByUser,
    },
};
use infrastructure::{
    database::postgres::repositories::{
        channel::PostgresChannelRepository, server::PostgresServerRepository,
        user::PostgresUserRepository, verification_code::PostgresVerificationCodeRepository,
    },
    hasher::bcrypt_hasher::BcryptHasher,
    mailer::smtp_mailer::SmtpMailer,
    voice::livekit_service::LiveKitService,
};

pub struct AppState {
    create_room_usecase:
        CreateChannel<PostgresChannelRepository, BcryptHasher, PostgresServerRepository>,
    join_room_usecase: JoinChannel<
        LiveKitService,
        PostgresChannelRepository,
        PostgresUserRepository,
        BcryptHasher,
    >,
    delete_room_usecase: DeleteChannel<PostgresChannelRepository>,

    register_user_usecase: RegisterUser<
        PostgresUserRepository,
        BcryptHasher,
        SmtpMailer,
        PostgresVerificationCodeRepository,
    >,
    login_user_usecase: LoginUser<PostgresUserRepository, BcryptHasher>,

    create_server_usecase: CreateServer<PostgresServerRepository>,
    delete_server_usecase: DeleteServer<PostgresServerRepository>,
    list_servers_by_user_usecase: ListServersByUser<PostgresServerRepository>,
    list_server_members_usecase: ListServerMembers<PostgresServerRepository>,
    list_server_rooms_usecase: ListServerChannels<PostgresServerRepository>,

    jwt_secret: String,
}

impl AppState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        create_room_usecase: CreateChannel<
            PostgresChannelRepository,
            BcryptHasher,
            PostgresServerRepository,
        >,
        join_room_usecase: JoinChannel<
            LiveKitService,
            PostgresChannelRepository,
            PostgresUserRepository,
            BcryptHasher,
        >,
        delete_room_usecase: DeleteChannel<PostgresChannelRepository>,
        register_user_usecase: RegisterUser<
            PostgresUserRepository,
            BcryptHasher,
            SmtpMailer,
            PostgresVerificationCodeRepository,
        >,
        login_user_usecase: LoginUser<PostgresUserRepository, BcryptHasher>,
        create_server_usecase: CreateServer<PostgresServerRepository>,
        delete_server_usecase: DeleteServer<PostgresServerRepository>,
        list_servers_by_user_usecase: ListServersByUser<PostgresServerRepository>,
        list_server_members_usecase: ListServerMembers<PostgresServerRepository>,
        list_server_rooms_usecase: ListServerChannels<PostgresServerRepository>,
        jwt_secret: String,
    ) -> Self {
        Self {
            create_room_usecase,
            join_room_usecase,
            delete_room_usecase,
            register_user_usecase,
            login_user_usecase,
            create_server_usecase,
            delete_server_usecase,
            list_servers_by_user_usecase,
            list_server_members_usecase,
            list_server_rooms_usecase,
            jwt_secret,
        }
    }

    pub fn create_room_usecase(
        &self,
    ) -> &CreateChannel<PostgresChannelRepository, BcryptHasher, PostgresServerRepository> {
        &self.create_room_usecase
    }

    pub fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }

    pub fn register_user_usecase(
        &self,
    ) -> &RegisterUser<
        PostgresUserRepository,
        BcryptHasher,
        SmtpMailer,
        PostgresVerificationCodeRepository,
    > {
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

    pub fn delete_room_usecase(&self) -> &DeleteChannel<PostgresChannelRepository> {
        &self.delete_room_usecase
    }

    pub fn join_room_usecase(
        &self,
    ) -> &JoinChannel<LiveKitService, PostgresChannelRepository, PostgresUserRepository, BcryptHasher>
    {
        &self.join_room_usecase
    }

    pub fn list_servers_by_user_usecase(&self) -> &ListServersByUser<PostgresServerRepository> {
        &self.list_servers_by_user_usecase
    }

    pub fn list_server_members_usecase(&self) -> &ListServerMembers<PostgresServerRepository> {
        &self.list_server_members_usecase
    }

    pub fn list_server_rooms_usecase(&self) -> &ListServerChannels<PostgresServerRepository> {
        &self.list_server_rooms_usecase
    }
}
