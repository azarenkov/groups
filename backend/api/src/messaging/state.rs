use application::usecases::{
    attachment::{
        get_attachment::GetAttachment, list_message_attachments::ListMessageAttachments,
    },
    call::log_call::LogCall,
    channel::{
        create_channel::CreateChannel, delete_channel::DeleteChannel,
        delete_channel_message::DeleteChannelMessage,
        edit_channel_message::EditChannelMessage, join_text_channel::JoinTextChannel,
        join_voice_channel::JoinVoiceChannel, list_channel_messages::ListChannelMessages,
        list_channel_participants::ListChannelParticipants, mute_channel::MuteChannel,
        search_channel_messages::SearchChannelMessages,
        send_channel_message::SendChannelMessage, unmute_channel::UnmuteChannel,
        update_stream_viewer::UpdateStreamViewer,
    },
    dm::{
        delete_dm_message::DeleteDmMessage, edit_dm_message::EditDmMessage,
        list_dm_messages::ListDmMessages, list_dm_previews::ListDmPreviews,
        search_dm_messages::SearchDmMessages, send_dm_message::SendDmMessage,
    },
    friendship::{
        accept_friend_request::AcceptFriendRequest, list_friends::ListFriends,
        remove_friendship::RemoveFriendship, send_friend_request::SendFriendRequest,
    },
    read_state::{
        get_channel_read_receipts::GetChannelReadReceipts,
        get_dm_read_receipt::GetDmReadReceipt, get_unread_counts::GetUnreadCounts,
        mark_channel_read::MarkChannelRead, mark_dm_read::MarkDmRead,
    },
    server::{
        accept_server_invite::AcceptServerInvite, create_server::CreateServer,
        decline_server_invite::DeclineServerInvite, delete_server::DeleteServer,
        get_voice_presence::GetVoicePresence,
        invite_codes::{CreateInviteCode, ListInviteCodes, RedeemInviteCode, RevokeInviteCode},
        leave_server::LeaveServer, list_sent_server_invites::ListSentServerInvites,
        list_server_channels::ListServerChannels, list_server_members::ListServerMembers,
        list_servers_by_user::ListServersByUser, send_server_invite::SendServerInvite,
        update_server::UpdateServer,
    },
};
use infrastructure::{
    database::postgres::repositories::{
        channel::PostgresChannelRepository,
        dm_message::PostgresDmMessageRepository, friendship::PostgresFriendshipRepository,
        message::PostgresMessageRepository, read_state::PostgresReadStateRepository,
        server::PostgresServerRepository, user_profile::PostgresUserProfileRepository,
    },
    hasher::bcrypt_hasher::BcryptHasher,
    media_client::claims::MediaClaimsClient,
    storage::minio_storage::MinioStorage,
    voice::livekit_service::LiveKitService,
};

pub struct AppState {
    pub create_channel_usecase:
        CreateChannel<PostgresChannelRepository, BcryptHasher, PostgresServerRepository>,
    pub join_voice_channel_usecase: JoinVoiceChannel<
        LiveKitService,
        PostgresChannelRepository,
        PostgresUserProfileRepository,
        BcryptHasher,
    >,
    pub mute_channel_usecase: MuteChannel<PostgresChannelRepository, PostgresUserProfileRepository>,
    pub unmute_channel_usecase: UnmuteChannel<PostgresChannelRepository, PostgresUserProfileRepository>,
    pub delete_channel_usecase: DeleteChannel<PostgresChannelRepository, LiveKitService>,
    pub list_channel_participants_usecase:
        ListChannelParticipants<PostgresChannelRepository, PostgresServerRepository>,
    pub join_text_channel_usecase:
        JoinTextChannel<PostgresChannelRepository, PostgresServerRepository, BcryptHasher>,
    pub send_channel_message_usecase: SendChannelMessage<
        PostgresMessageRepository,
        PostgresChannelRepository,
        PostgresServerRepository,
        MediaClaimsClient,
    >,
    pub edit_channel_message_usecase: EditChannelMessage<
        PostgresMessageRepository,
        PostgresChannelRepository,
        PostgresServerRepository,
        MediaClaimsClient,
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
    pub search_channel_messages_usecase: SearchChannelMessages<
        PostgresMessageRepository,
        PostgresChannelRepository,
        PostgresServerRepository,
    >,
    pub list_message_attachments_usecase:
        ListMessageAttachments<PostgresMessageRepository, PostgresDmMessageRepository>,
    pub get_attachment_usecase: GetAttachment<PostgresMessageRepository, MinioStorage>,
    pub send_friend_request_usecase:
        SendFriendRequest<PostgresFriendshipRepository, PostgresUserProfileRepository>,
    pub accept_friend_request_usecase: AcceptFriendRequest<PostgresFriendshipRepository>,
    pub remove_friendship_usecase: RemoveFriendship<PostgresFriendshipRepository>,
    pub list_friends_usecase: ListFriends<PostgresFriendshipRepository>,
    pub mark_channel_read_usecase:
        MarkChannelRead<PostgresReadStateRepository, PostgresChannelRepository>,
    pub mark_dm_read_usecase:
        MarkDmRead<PostgresReadStateRepository, PostgresFriendshipRepository>,
    pub get_unread_counts_usecase: GetUnreadCounts<PostgresReadStateRepository>,
    pub get_dm_read_receipt_usecase:
        GetDmReadReceipt<PostgresReadStateRepository, PostgresFriendshipRepository>,
    pub get_channel_read_receipts_usecase:
        GetChannelReadReceipts<PostgresReadStateRepository, PostgresChannelRepository>,
    pub log_call_usecase: LogCall<PostgresDmMessageRepository, PostgresFriendshipRepository>,
    pub nats_core: async_nats::Client,
    pub presence_preference_subject: String,
    pub send_dm_message_usecase: SendDmMessage<
        PostgresDmMessageRepository,
        PostgresFriendshipRepository,
        MediaClaimsClient,
    >,
    pub edit_dm_message_usecase: EditDmMessage<PostgresDmMessageRepository, MediaClaimsClient>,
    pub delete_dm_message_usecase: DeleteDmMessage<PostgresDmMessageRepository>,
    pub list_dm_previews_usecase: ListDmPreviews<PostgresDmMessageRepository>,
    pub list_dm_messages_usecase:
        ListDmMessages<PostgresDmMessageRepository, PostgresFriendshipRepository>,
    pub search_dm_messages_usecase:
        SearchDmMessages<PostgresDmMessageRepository, PostgresFriendshipRepository>,
    pub send_server_invite_usecase: SendServerInvite<
        PostgresDmMessageRepository,
        PostgresFriendshipRepository,
        PostgresServerRepository,
    >,
    pub accept_server_invite_usecase:
        AcceptServerInvite<PostgresDmMessageRepository, PostgresServerRepository>,
    pub decline_server_invite_usecase: DeclineServerInvite<PostgresDmMessageRepository>,
    pub list_sent_server_invites_usecase:
        ListSentServerInvites<PostgresDmMessageRepository, PostgresServerRepository>,
    pub create_server_usecase: CreateServer<PostgresServerRepository>,
    pub delete_server_usecase:
        DeleteServer<PostgresServerRepository, PostgresChannelRepository, LiveKitService>,
    pub update_server_usecase: UpdateServer<PostgresServerRepository>,
    pub create_invite_code_usecase: CreateInviteCode<PostgresServerRepository>,
    pub list_invite_codes_usecase: ListInviteCodes<PostgresServerRepository>,
    pub revoke_invite_code_usecase: RevokeInviteCode<PostgresServerRepository>,
    pub redeem_invite_code_usecase: RedeemInviteCode<PostgresServerRepository>,
    pub leave_server_usecase:
        LeaveServer<PostgresServerRepository, PostgresChannelRepository, LiveKitService>,
    pub list_servers_by_user_usecase: ListServersByUser<PostgresServerRepository>,
    pub list_server_members_usecase: ListServerMembers<PostgresServerRepository>,
    pub list_server_channels_usecase: ListServerChannels<PostgresServerRepository>,
    pub get_voice_presence_usecase:
        GetVoicePresence<PostgresServerRepository, PostgresChannelRepository>,
    pub update_stream_viewer_usecase:
        UpdateStreamViewer<PostgresServerRepository, PostgresChannelRepository>,
}
