use application::usecases::{
    auth::{
        confirm_registration::ConfirmRegistration, login::LoginUser, logout::Logout,
        refresh_tokens::RefreshTokens, register::RegisterUser,
    },
    channel::{
        create_channel::CreateChannel, delete_channel::DeleteChannel,
        join_voice_channel::JoinVoiceChannel, leave_voice_channel::LeaveVoiceChannel,
        mute_channel::MuteChannel, unmute_channel::UnmuteChannel,
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
    database::postgres::repositories::{
        channel::PostgresChannelRepository, refresh_token::PostgresRefreshTokenRepository,
        server::PostgresServerRepository, user::PostgresUserRepository,
        verification_code::PostgresVerificationCodeRepository,
    },
    hasher::bcrypt_hasher::BcryptHasher,
    voice::{livekit_service::LiveKitService, livekit_webhook_receiver::LiveKitWebhookReceiver},
};

pub struct AppState {
    pub create_channel_usecase:
        CreateChannel<PostgresChannelRepository, BcryptHasher, PostgresServerRepository>,
    pub join_voice_channel_usecase: JoinVoiceChannel<
        LiveKitService,
        PostgresChannelRepository,
        PostgresUserRepository,
        BcryptHasher,
    >,
    pub leave_voice_channel_usecase: LeaveVoiceChannel<PostgresChannelRepository>,
    pub mute_channel_usecase: MuteChannel<PostgresChannelRepository, PostgresUserRepository>,
    pub unmute_channel_usecase: UnmuteChannel<PostgresChannelRepository, PostgresUserRepository>,

    pub delete_channel_usecase: DeleteChannel<PostgresChannelRepository>,

    pub register_user_usecase:
        RegisterUser<PostgresUserRepository, BcryptHasher, JwtTokenIssuer>,
    pub login_user_usecase: LoginUser<
        PostgresUserRepository,
        BcryptHasher,
        PostgresRefreshTokenRepository,
        JwtTokenIssuer,
    >,
    pub confirm_registration_usecase: ConfirmRegistration<
        PostgresUserRepository,
        BcryptHasher,
        PostgresVerificationCodeRepository,
    >,
    pub refresh_tokens_usecase: RefreshTokens<PostgresRefreshTokenRepository, JwtTokenIssuer>,
    pub logout_usecase: Logout<PostgresRefreshTokenRepository, JwtTokenIssuer>,

    pub handle_livekit_webhook_usecase:
        HandleVoiceWebhook<LiveKitWebhookReceiver, PostgresChannelRepository>,

    pub create_server_usecase: CreateServer<PostgresServerRepository>,
    pub delete_server_usecase: DeleteServer<PostgresServerRepository>,
    pub list_servers_by_user_usecase: ListServersByUser<PostgresServerRepository>,
    pub list_server_members_usecase: ListServerMembers<PostgresServerRepository>,
    pub list_server_channels_usecase: ListServerChannels<PostgresServerRepository>,

    pub jwt_secret: String,
    pub trust_forwarded_headers: bool,
}

impl AppState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        create_channel_usecase: CreateChannel<
            PostgresChannelRepository,
            BcryptHasher,
            PostgresServerRepository,
        >,
        join_voice_channel_usecase: JoinVoiceChannel<
            LiveKitService,
            PostgresChannelRepository,
            PostgresUserRepository,
            BcryptHasher,
        >,
        leave_voice_channel_usecase: LeaveVoiceChannel<PostgresChannelRepository>,
        mute_channel_usecase: MuteChannel<PostgresChannelRepository, PostgresUserRepository>,
        unmute_channel_usecase: UnmuteChannel<PostgresChannelRepository, PostgresUserRepository>,
        delete_channel_usecase: DeleteChannel<PostgresChannelRepository>,
        register_user_usecase: RegisterUser<
            PostgresUserRepository,
            BcryptHasher,
            JwtTokenIssuer,
        >,
        login_user_usecase: LoginUser<
            PostgresUserRepository,
            BcryptHasher,
            PostgresRefreshTokenRepository,
            JwtTokenIssuer,
        >,
        confirm_registration_usecase: ConfirmRegistration<
            PostgresUserRepository,
            BcryptHasher,
            PostgresVerificationCodeRepository,
        >,
        refresh_tokens_usecase: RefreshTokens<PostgresRefreshTokenRepository, JwtTokenIssuer>,
        logout_usecase: Logout<PostgresRefreshTokenRepository, JwtTokenIssuer>,
        handle_livekit_webhook_usecase: HandleVoiceWebhook<
            LiveKitWebhookReceiver,
            PostgresChannelRepository,
        >,
        create_server_usecase: CreateServer<PostgresServerRepository>,
        delete_server_usecase: DeleteServer<PostgresServerRepository>,
        list_servers_by_user_usecase: ListServersByUser<PostgresServerRepository>,
        list_server_members_usecase: ListServerMembers<PostgresServerRepository>,
        list_server_channels_usecase: ListServerChannels<PostgresServerRepository>,
        jwt_secret: String,
        trust_forwarded_headers: bool,
    ) -> Self {
        Self {
            create_channel_usecase,
            join_voice_channel_usecase,
            leave_voice_channel_usecase,
            mute_channel_usecase,
            unmute_channel_usecase,
            delete_channel_usecase,
            register_user_usecase,
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
            jwt_secret,
            trust_forwarded_headers,
        }
    }
}
