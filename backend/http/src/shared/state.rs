use application::usecases::{
    auth::{
        confirm_registration::ConfirmRegistration,
        issue_verification_code::IssueVerificationCode, login::LoginUser, logout::Logout,
        refresh_tokens::RefreshTokens, register::RegisterUser,
    },
    channel::{
        create_channel::CreateChannel, delete_channel::DeleteChannel,
        delete_channel_message::DeleteChannelMessage,
        edit_channel_message::EditChannelMessage, join_text_channel::JoinTextChannel,
        join_voice_channel::JoinVoiceChannel,
        list_channel_messages::ListChannelMessages,
        list_channel_participants::ListChannelParticipants, mute_channel::MuteChannel,
        send_channel_message::SendChannelMessage, unmute_channel::UnmuteChannel,
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
        channel::PostgresChannelRepository, message::PostgresMessageRepository,
        refresh_token::PostgresRefreshTokenRepository, server::PostgresServerRepository,
        user::PostgresUserRepository, verification_code::PostgresVerificationCodeRepository,
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
    pub mute_channel_usecase: MuteChannel<PostgresChannelRepository, PostgresUserRepository>,
    pub unmute_channel_usecase: UnmuteChannel<PostgresChannelRepository, PostgresUserRepository>,

    pub delete_channel_usecase: DeleteChannel<PostgresChannelRepository, LiveKitService>,
    pub list_channel_participants_usecase: ListChannelParticipants<PostgresChannelRepository, PostgresServerRepository>,

    pub join_text_channel_usecase:
        JoinTextChannel<PostgresChannelRepository, PostgresServerRepository, BcryptHasher>,
    pub send_channel_message_usecase: SendChannelMessage<
        PostgresMessageRepository,
        PostgresChannelRepository,
        PostgresServerRepository,
    >,
    pub edit_channel_message_usecase: EditChannelMessage<
        PostgresMessageRepository,
        PostgresChannelRepository,
        PostgresServerRepository,
    >,
    pub delete_channel_message_usecase: DeleteChannelMessage<
        PostgresMessageRepository,
        PostgresChannelRepository,
        PostgresServerRepository,
    >,
    pub list_channel_messages_usecase: ListChannelMessages<
        PostgresMessageRepository,
        PostgresChannelRepository,
        PostgresServerRepository,
    >,

    pub register_user_usecase:
        RegisterUser<PostgresUserRepository, BcryptHasher, JwtTokenIssuer>,
    pub issue_verification_code_usecase: IssueVerificationCode<
        PostgresUserRepository,
        PostgresVerificationCodeRepository,
        BcryptHasher,
    >,
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
    pub refresh_tokens_usecase: RefreshTokens<PostgresRefreshTokenRepository, PostgresUserRepository, JwtTokenIssuer>,
    pub logout_usecase: Logout<PostgresRefreshTokenRepository, JwtTokenIssuer>,

    pub handle_livekit_webhook_usecase:
        HandleVoiceWebhook<LiveKitWebhookReceiver, PostgresChannelRepository>,

    pub create_server_usecase: CreateServer<PostgresServerRepository>,
    pub delete_server_usecase: DeleteServer<PostgresServerRepository, PostgresChannelRepository, LiveKitService>,
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
        mute_channel_usecase: MuteChannel<PostgresChannelRepository, PostgresUserRepository>,
        unmute_channel_usecase: UnmuteChannel<PostgresChannelRepository, PostgresUserRepository>,
        delete_channel_usecase: DeleteChannel<PostgresChannelRepository, LiveKitService>,
        list_channel_participants_usecase: ListChannelParticipants<PostgresChannelRepository, PostgresServerRepository>,
        join_text_channel_usecase: JoinTextChannel<
            PostgresChannelRepository,
            PostgresServerRepository,
            BcryptHasher,
        >,
        send_channel_message_usecase: SendChannelMessage<
            PostgresMessageRepository,
            PostgresChannelRepository,
            PostgresServerRepository,
        >,
        edit_channel_message_usecase: EditChannelMessage<
            PostgresMessageRepository,
            PostgresChannelRepository,
            PostgresServerRepository,
        >,
        delete_channel_message_usecase: DeleteChannelMessage<
            PostgresMessageRepository,
            PostgresChannelRepository,
            PostgresServerRepository,
        >,
        list_channel_messages_usecase: ListChannelMessages<
            PostgresMessageRepository,
            PostgresChannelRepository,
            PostgresServerRepository,
        >,
        register_user_usecase: RegisterUser<
            PostgresUserRepository,
            BcryptHasher,
            JwtTokenIssuer,
        >,
        issue_verification_code_usecase: IssueVerificationCode<
            PostgresUserRepository,
            PostgresVerificationCodeRepository,
            BcryptHasher,
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
        refresh_tokens_usecase: RefreshTokens<PostgresRefreshTokenRepository, PostgresUserRepository, JwtTokenIssuer>,
        logout_usecase: Logout<PostgresRefreshTokenRepository, JwtTokenIssuer>,
        handle_livekit_webhook_usecase: HandleVoiceWebhook<
            LiveKitWebhookReceiver,
            PostgresChannelRepository,
        >,
        create_server_usecase: CreateServer<PostgresServerRepository>,
        delete_server_usecase: DeleteServer<PostgresServerRepository, PostgresChannelRepository, LiveKitService>,
        list_servers_by_user_usecase: ListServersByUser<PostgresServerRepository>,
        list_server_members_usecase: ListServerMembers<PostgresServerRepository>,
        list_server_channels_usecase: ListServerChannels<PostgresServerRepository>,
        jwt_secret: String,
        trust_forwarded_headers: bool,
    ) -> Self {
        Self {
            create_channel_usecase,
            join_voice_channel_usecase,
            mute_channel_usecase,
            unmute_channel_usecase,
            delete_channel_usecase,
            list_channel_participants_usecase,
            join_text_channel_usecase,
            send_channel_message_usecase,
            edit_channel_message_usecase,
            delete_channel_message_usecase,
            list_channel_messages_usecase,
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
            jwt_secret,
            trust_forwarded_headers,
        }
    }
}
