use messaging::usecases::block::{block_user::BlockUser, list_blocked_users::ListBlockedUsers, unblock_user::UnblockUser};
use messaging::usecases::{
    privacy::{get_privacy::GetPrivacy, update_privacy::UpdatePrivacy},
    attachment::{
        get_attachment::GetAttachment, list_message_attachments::ListMessageAttachments,
    },
    call::log_call::LogCall,
    channel::{
        create_channel::CreateChannel, delete_channel::DeleteChannel,
        delete_channel_message::DeleteChannelMessage,
        edit_channel_message::EditChannelMessage, join_text_channel::JoinTextChannel,
        join_voice_channel::JoinVoiceChannel, list_channel_messages::ListChannelMessages,
        deafen_channel::DeafenChannel, list_channel_participants::ListChannelParticipants, mute_channel::MuteChannel,
        undeafen_channel::UndeafenChannel,
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
    notification::{
        list_notification_preferences::ListNotificationPreferences,
        set_notification_preference::SetNotificationPreference,
    },
    role::{
        assign_role::AssignRole, create_role::CreateRole, delete_role::DeleteRole,
        list_roles::ListRoles, revoke_role::RevokeRole, swap_roles::SwapRoles,
        update_role::UpdateRole,
    },
    read_state::{
        get_channel_read_receipts::GetChannelReadReceipts,
        get_dm_read_receipt::GetDmReadReceipt, get_unread_counts::GetUnreadCounts,
        mark_channel_read::MarkChannelRead, mark_dm_read::MarkDmRead,
    },
    server::{
        accept_server_invite::AcceptServerInvite, can_manage_server::CanManageServer,
        create_server::CreateServer,
        decline_server_invite::DeclineServerInvite, delete_server::DeleteServer,
        get_home_overview::GetHomeOverview, get_user_profile::GetUserProfile, get_voice_presence::GetVoicePresence,
        invite_links::{CreateInviteLink, JoinByInviteLink, ListInviteLinks, PreviewInviteLink, RevokeInviteLink},
        kick_member::KickMember, leave_server::LeaveServer,
        list_sent_server_invites::ListSentServerInvites,
        list_server_channels::ListServerChannels, list_server_members::ListServerMembers,
        list_servers_by_user::ListServersByUser, send_server_invite::SendServerInvite,
        update_server::UpdateServer,
    },
};
use infrastructure_common::moderation::text_detector::TextModerationClient;
use infrastructure_common::{
    hasher::bcrypt_hasher::BcryptHasher,
    storage::minio_storage::MinioStorage,
    voice::livekit_service::LiveKitService,
};
use messaging_infrastructure::{
    database::postgres::repositories::{
        channel::PostgresChannelRepository,
        dm_message::PostgresDmMessageRepository, friendship::PostgresFriendshipRepository,
        message::PostgresMessageRepository,
        notification_preference::PostgresNotificationPreferenceRepository,
        read_state::PostgresReadStateRepository,
        server::PostgresServerRepository,
        server_role::PostgresServerRoleRepository,
        channel_unit_of_work::PostgresChannelUnitOfWork,
        dm_unit_of_work::PostgresDmUnitOfWork,
        friendship_unit_of_work::PostgresFriendshipUnitOfWork,
        message_unit_of_work::PostgresMessageUnitOfWork,
        server_unit_of_work::PostgresServerUnitOfWork,
        privacy::PostgresPrivacyRepository, user_profile::PostgresUserProfileRepository,
    },
    media_client::claims::MediaClaimsClient,
    media_client::voice_calls::VoiceCallsClient,
    media_client::identity_users::IdentityUsersClient,
};

pub struct AppState {
    pub create_channel_usecase:
        CreateChannel<BcryptHasher, PostgresServerRepository, PostgresServerUnitOfWork>,
    pub join_voice_channel_usecase: JoinVoiceChannel<
        LiveKitService,
        PostgresChannelRepository,
        PostgresUserProfileRepository,
        BcryptHasher,
        PostgresServerRepository,
    >,
    pub mute_channel_usecase: MuteChannel<
        PostgresChannelRepository,
        PostgresUserProfileRepository,
        PostgresServerUnitOfWork,
    >,
    pub unmute_channel_usecase: UnmuteChannel<
        PostgresChannelRepository,
        PostgresUserProfileRepository,
        PostgresServerUnitOfWork,
    >,
    pub deafen_channel_usecase: DeafenChannel<
        PostgresChannelRepository,
        PostgresUserProfileRepository,
        PostgresServerUnitOfWork,
    >,
    pub undeafen_channel_usecase: UndeafenChannel<
        PostgresChannelRepository,
        PostgresUserProfileRepository,
        PostgresServerUnitOfWork,
    >,
    pub delete_channel_usecase:
        DeleteChannel<PostgresChannelRepository, LiveKitService, PostgresServerUnitOfWork>,
    pub list_channel_participants_usecase:
        ListChannelParticipants<
            PostgresChannelRepository,
            PostgresServerRepository,
            PostgresFriendshipRepository,
        >,
    pub join_text_channel_usecase: JoinTextChannel<
        PostgresChannelRepository,
        PostgresServerRepository,
        PostgresChannelUnitOfWork,
        BcryptHasher,
    >,
    pub send_channel_message_usecase: SendChannelMessage<
        PostgresMessageRepository,
        PostgresChannelRepository,
        PostgresServerRepository,
        PostgresMessageUnitOfWork,
        MediaClaimsClient,
        IdentityUsersClient,
    >,
    pub edit_channel_message_usecase: EditChannelMessage<
        PostgresMessageRepository,
        PostgresChannelRepository,
        PostgresServerRepository,
        PostgresMessageUnitOfWork,
        MediaClaimsClient,
        IdentityUsersClient,
    >,
    pub delete_channel_message_usecase: DeleteChannelMessage<
        PostgresMessageRepository,
        PostgresChannelRepository,
        PostgresServerUnitOfWork,
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
    pub get_attachment_usecase:
        GetAttachment<PostgresMessageRepository, MinioStorage, MediaClaimsClient>,
    pub send_friend_request_usecase:
        SendFriendRequest<PostgresFriendshipUnitOfWork, PostgresUserProfileRepository, PostgresPrivacyRepository>,
    pub get_privacy_usecase: GetPrivacy<PostgresPrivacyRepository>,
    pub update_privacy_usecase: UpdatePrivacy<PostgresPrivacyRepository>,
    pub accept_friend_request_usecase: AcceptFriendRequest<PostgresFriendshipUnitOfWork>,
    pub remove_friendship_usecase: RemoveFriendship<PostgresFriendshipUnitOfWork>,
    pub list_friends_usecase: ListFriends<PostgresFriendshipRepository>,
    pub block_user_usecase: BlockUser<PostgresFriendshipUnitOfWork, PostgresUserProfileRepository>,
    pub unblock_user_usecase: UnblockUser<PostgresFriendshipUnitOfWork>,
    pub list_blocked_users_usecase: ListBlockedUsers<PostgresFriendshipRepository>,
    pub mark_channel_read_usecase:
        MarkChannelRead<PostgresReadStateRepository, PostgresChannelRepository>,
    pub mark_dm_read_usecase:
        MarkDmRead<PostgresReadStateRepository, PostgresFriendshipRepository, PostgresDmMessageRepository>,
    pub get_unread_counts_usecase: GetUnreadCounts<PostgresReadStateRepository>,
    pub get_dm_read_receipt_usecase:
        GetDmReadReceipt<PostgresReadStateRepository, PostgresFriendshipRepository, PostgresDmMessageRepository>,
    pub get_channel_read_receipts_usecase:
        GetChannelReadReceipts<PostgresReadStateRepository, PostgresChannelRepository>,
    pub list_roles_usecase: ListRoles<PostgresServerRepository, PostgresServerRoleRepository>,
    pub can_manage_server_usecase:
        CanManageServer<PostgresServerRepository, PostgresServerRoleRepository>,
    pub create_role_usecase: CreateRole<PostgresServerUnitOfWork>,
    pub update_role_usecase: UpdateRole<PostgresServerUnitOfWork>,
    pub delete_role_usecase: DeleteRole<PostgresServerUnitOfWork>,
    pub assign_role_usecase: AssignRole<PostgresServerUnitOfWork>,
    pub revoke_role_usecase: RevokeRole<PostgresServerUnitOfWork>,
    pub swap_roles_usecase: SwapRoles<PostgresServerUnitOfWork>,
    pub list_notification_preferences_usecase:
        ListNotificationPreferences<PostgresNotificationPreferenceRepository>,
    pub set_notification_preference_usecase: SetNotificationPreference<
        PostgresNotificationPreferenceRepository,
        PostgresChannelRepository,
        PostgresFriendshipRepository,
    >,
    pub log_call_usecase: LogCall<PostgresDmUnitOfWork, VoiceCallsClient>,
    pub server_repository: std::sync::Arc<PostgresServerRepository>,
    pub message_repository: std::sync::Arc<PostgresMessageRepository>,
    pub dm_repository: std::sync::Arc<PostgresDmMessageRepository>,
    pub friendship_repository: std::sync::Arc<PostgresFriendshipRepository>,
    pub privacy_repository: std::sync::Arc<PostgresPrivacyRepository>,
    pub nats_core: async_nats::Client,
    pub presence_preference_subject: String,
    pub send_dm_message_usecase: SendDmMessage<
        PostgresDmMessageRepository,
        PostgresDmUnitOfWork,
        PostgresFriendshipRepository,
        MediaClaimsClient,
        PostgresPrivacyRepository,
        PostgresUserProfileRepository,
    >,
    pub edit_dm_message_usecase: EditDmMessage<
        PostgresDmMessageRepository,
        PostgresDmUnitOfWork,
        MediaClaimsClient,
        PostgresFriendshipRepository,
        PostgresPrivacyRepository,
        PostgresUserProfileRepository,
    >,
    pub delete_dm_message_usecase: DeleteDmMessage<PostgresDmMessageRepository, PostgresDmUnitOfWork>,
    pub list_dm_previews_usecase: ListDmPreviews<PostgresDmMessageRepository>,
    pub list_dm_messages_usecase: ListDmMessages<PostgresDmMessageRepository>,
    pub search_dm_messages_usecase: SearchDmMessages<PostgresDmMessageRepository>,
    pub send_server_invite_usecase: SendServerInvite<
        PostgresDmMessageRepository,
        PostgresFriendshipRepository,
        PostgresServerRepository,
        PostgresServerUnitOfWork,
        PostgresPrivacyRepository,
        PostgresUserProfileRepository,
    >,
    pub accept_server_invite_usecase:
        AcceptServerInvite<PostgresDmMessageRepository, PostgresServerUnitOfWork>,
    pub decline_server_invite_usecase:
        DeclineServerInvite<PostgresDmMessageRepository, PostgresDmUnitOfWork>,
    pub list_sent_server_invites_usecase:
        ListSentServerInvites<PostgresDmMessageRepository, PostgresServerRepository>,
    pub create_server_usecase: CreateServer<PostgresServerUnitOfWork, TextModerationClient>,
    pub delete_server_usecase: DeleteServer<
        PostgresServerRepository,
        PostgresChannelRepository,
        PostgresServerUnitOfWork,
        LiveKitService,
    >,
    pub update_server_usecase: UpdateServer<PostgresServerUnitOfWork, TextModerationClient>,
    pub create_invite_link_usecase: CreateInviteLink<PostgresServerUnitOfWork>,
    pub list_invite_links_usecase:
        ListInviteLinks<PostgresServerRepository, PostgresServerUnitOfWork>,
    pub revoke_invite_link_usecase: RevokeInviteLink<PostgresServerUnitOfWork>,
    pub preview_invite_link_usecase: PreviewInviteLink<PostgresServerRepository>,
    pub join_by_invite_link_usecase:
        JoinByInviteLink<PostgresServerRepository, PostgresServerUnitOfWork>,
    pub public_app_url: String,
    pub leave_server_usecase:
        LeaveServer<PostgresServerUnitOfWork>,
    pub kick_member_usecase:
        KickMember<PostgresServerUnitOfWork>,
    pub list_servers_by_user_usecase: ListServersByUser<PostgresServerRepository>,
    pub list_server_members_usecase: ListServerMembers<PostgresServerRepository>,
    pub list_server_channels_usecase: ListServerChannels<PostgresServerRepository>,
    pub get_home_overview_usecase: GetHomeOverview<PostgresChannelRepository>,
    pub get_user_profile_usecase:
        GetUserProfile<PostgresUserProfileRepository, PostgresServerRepository, PostgresChannelRepository>,
    pub get_voice_presence_usecase:
        GetVoicePresence<PostgresServerRepository, PostgresChannelRepository>,
    pub update_stream_viewer_usecase:
        UpdateStreamViewer<PostgresServerRepository, PostgresChannelRepository>,
}
