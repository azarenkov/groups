use application::usecases::{
    auth::{login::LoginUser, register::RegisterUser},
    channel::{
        create_channel::CreateChannel, delete_channel::DeleteChannel, join_channel::JoinChannel,
        leave_channel::LeaveChannel, mute_channel::MuteChannel, unmute_channel::UnmuteChannel,
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
    pub create_channel_usecase:
        CreateChannel<PostgresChannelRepository, BcryptHasher, PostgresServerRepository>,
    pub join_channel_usecase: JoinChannel<
        LiveKitService,
        PostgresChannelRepository,
        PostgresUserRepository,
        BcryptHasher,
    >,
    pub leave_channel_usecase: LeaveChannel<PostgresChannelRepository>,
    pub mute_channel_usecase: MuteChannel<PostgresChannelRepository>,
    pub unmute_channel_usecase: UnmuteChannel<PostgresChannelRepository>,

    pub delete_channel_usecase: DeleteChannel<PostgresChannelRepository>,

    pub register_user_usecase: RegisterUser<
        PostgresUserRepository,
        BcryptHasher,
        SmtpMailer,
        PostgresVerificationCodeRepository,
    >,
    pub login_user_usecase: LoginUser<PostgresUserRepository, BcryptHasher>,

    pub create_server_usecase: CreateServer<PostgresServerRepository>,
    pub delete_server_usecase: DeleteServer<PostgresServerRepository>,
    pub list_servers_by_user_usecase: ListServersByUser<PostgresServerRepository>,
    pub list_server_members_usecase: ListServerMembers<PostgresServerRepository>,
    pub list_server_channels_usecase: ListServerChannels<PostgresServerRepository>,

    pub jwt_secret: String,
}

impl AppState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        create_channel_usecase: CreateChannel<
            PostgresChannelRepository,
            BcryptHasher,
            PostgresServerRepository,
        >,
        join_channel_usecase: JoinChannel<
            LiveKitService,
            PostgresChannelRepository,
            PostgresUserRepository,
            BcryptHasher,
        >,
        leave_channel_usecase: LeaveChannel<PostgresChannelRepository>,
        mute_channel_usecase: MuteChannel<PostgresChannelRepository>,
        unmute_channel_usecase: UnmuteChannel<PostgresChannelRepository>,
        delete_channel_usecase: DeleteChannel<PostgresChannelRepository>,
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
        list_server_channels_usecase: ListServerChannels<PostgresServerRepository>,
        jwt_secret: String,
    ) -> Self {
        Self {
            create_channel_usecase,
            join_channel_usecase,
            leave_channel_usecase,
            mute_channel_usecase,
            unmute_channel_usecase,
            delete_channel_usecase,
            register_user_usecase,
            login_user_usecase,
            create_server_usecase,
            delete_server_usecase,
            list_servers_by_user_usecase,
            list_server_members_usecase,
            list_server_channels_usecase,
            jwt_secret,
        }
    }
}
