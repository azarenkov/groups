use std::sync::Arc;
use messaging::usecases::block::{block_user::BlockUser, list_blocked_users::ListBlockedUsers, unblock_user::UnblockUser};
use messaging::usecases::{
    privacy::{get_privacy::GetPrivacy, update_privacy::UpdatePrivacy},
    report::create_report::CreateReport,
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
    community::{
        accept_community_invite::AcceptCommunityInvite, can_manage_community::CanManageCommunity,
        create_community::CreateCommunity,
        decline_community_invite::DeclineCommunityInvite, delete_community::DeleteCommunity,
        get_home_overview::GetHomeOverview, get_user_profile::GetUserProfile, get_voice_presence::GetVoicePresence,
        invite_links::{CreateInviteLink, JoinByInviteLink, ListInviteLinks, PreviewInviteLink, RevokeInviteLink},
        kick_member::KickMember, leave_community::LeaveCommunity,
        list_sent_community_invites::ListSentCommunityInvites,
        list_community_channels::ListCommunityChannels, list_community_members::ListCommunityMembers,
        list_communities_by_user::ListCommunitiesByUser, send_community_invite::SendCommunityInvite,
        update_community::UpdateCommunity,
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
        community::PostgresCommunityRepository,
        community_role::PostgresCommunityRoleRepository,
        channel_unit_of_work::PostgresChannelUnitOfWork,
        dm_unit_of_work::PostgresDmUnitOfWork,
        friendship_unit_of_work::PostgresFriendshipUnitOfWork,
        message_unit_of_work::PostgresMessageUnitOfWork,
        community_unit_of_work::PostgresCommunityUnitOfWork,
        badge::PostgresBadgeRepository, privacy::PostgresPrivacyRepository,
        report::PostgresReportRepository,
        user_profile::PostgresUserProfileRepository,
    },
    media_client::claims::MediaClaimsClient,
    media_client::voice_calls::VoiceCallsClient,
    media_client::identity_users::IdentityUsersClient,
};

pub struct AppState {
    pub create_channel_usecase:
        CreateChannel<BcryptHasher, PostgresCommunityRepository, PostgresCommunityUnitOfWork>,
    pub join_voice_channel_usecase: JoinVoiceChannel<
        LiveKitService,
        PostgresChannelRepository,
        PostgresUserProfileRepository,
        BcryptHasher,
        PostgresCommunityRepository,
    >,
    pub mute_channel_usecase: MuteChannel<
        PostgresChannelRepository,
        PostgresUserProfileRepository,
        PostgresCommunityUnitOfWork,
    >,
    pub unmute_channel_usecase: UnmuteChannel<
        PostgresChannelRepository,
        PostgresUserProfileRepository,
        PostgresCommunityUnitOfWork,
    >,
    pub deafen_channel_usecase: DeafenChannel<
        PostgresChannelRepository,
        PostgresUserProfileRepository,
        PostgresCommunityUnitOfWork,
    >,
    pub undeafen_channel_usecase: UndeafenChannel<
        PostgresChannelRepository,
        PostgresUserProfileRepository,
        PostgresCommunityUnitOfWork,
    >,
    pub delete_channel_usecase:
        DeleteChannel<PostgresChannelRepository, LiveKitService, PostgresCommunityUnitOfWork>,
    pub list_channel_participants_usecase:
        ListChannelParticipants<
            PostgresChannelRepository,
            PostgresCommunityRepository,
            PostgresFriendshipRepository,
        >,
    pub join_text_channel_usecase: JoinTextChannel<
        PostgresChannelRepository,
        PostgresCommunityRepository,
        PostgresChannelUnitOfWork,
        BcryptHasher,
    >,
    pub send_channel_message_usecase: SendChannelMessage<
        PostgresMessageRepository,
        PostgresChannelRepository,
        PostgresCommunityRepository,
        PostgresMessageUnitOfWork,
        MediaClaimsClient,
        IdentityUsersClient,
    >,
    pub edit_channel_message_usecase: EditChannelMessage<
        PostgresMessageRepository,
        PostgresChannelRepository,
        PostgresCommunityRepository,
        PostgresMessageUnitOfWork,
        MediaClaimsClient,
        IdentityUsersClient,
    >,
    pub delete_channel_message_usecase: DeleteChannelMessage<
        PostgresMessageRepository,
        PostgresChannelRepository,
        PostgresCommunityUnitOfWork,
    >,
    pub list_channel_messages_usecase: ListChannelMessages<
        PostgresMessageRepository,
        PostgresChannelRepository,
        PostgresCommunityRepository,
    >,
    pub search_channel_messages_usecase: SearchChannelMessages<
        PostgresMessageRepository,
        PostgresChannelRepository,
        PostgresCommunityRepository,
    >,
    pub list_message_attachments_usecase:
        ListMessageAttachments<PostgresMessageRepository, PostgresDmMessageRepository>,
    pub get_attachment_usecase:
        GetAttachment<PostgresMessageRepository, MinioStorage, MediaClaimsClient>,
    pub send_friend_request_usecase:
        SendFriendRequest<PostgresFriendshipUnitOfWork, PostgresUserProfileRepository, PostgresPrivacyRepository>,
    pub create_report_usecase: CreateReport<
        PostgresReportRepository,
        PostgresUserProfileRepository,
        PostgresCommunityRepository,
        PostgresMessageRepository,
        PostgresDmMessageRepository,
        PostgresChannelRepository,
    >,
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
    pub list_roles_usecase: ListRoles<PostgresCommunityRepository, PostgresCommunityRoleRepository>,
    pub can_manage_community_usecase:
        CanManageCommunity<PostgresCommunityRepository, PostgresCommunityRoleRepository>,
    pub create_role_usecase: CreateRole<PostgresCommunityUnitOfWork>,
    pub update_role_usecase: UpdateRole<PostgresCommunityUnitOfWork>,
    pub delete_role_usecase: DeleteRole<PostgresCommunityUnitOfWork>,
    pub assign_role_usecase: AssignRole<PostgresCommunityUnitOfWork>,
    pub revoke_role_usecase: RevokeRole<PostgresCommunityUnitOfWork>,
    pub swap_roles_usecase: SwapRoles<PostgresCommunityUnitOfWork>,
    pub list_notification_preferences_usecase:
        ListNotificationPreferences<PostgresNotificationPreferenceRepository>,
    pub set_notification_preference_usecase: SetNotificationPreference<
        PostgresNotificationPreferenceRepository,
        PostgresChannelRepository,
        PostgresFriendshipRepository,
    >,
    pub log_call_usecase: LogCall<PostgresDmUnitOfWork, VoiceCallsClient>,
    pub community_repository: std::sync::Arc<PostgresCommunityRepository>,
    pub message_repository: std::sync::Arc<PostgresMessageRepository>,
    pub dm_repository: std::sync::Arc<PostgresDmMessageRepository>,
    pub friendship_repository: std::sync::Arc<PostgresFriendshipRepository>,
    pub privacy_repository: std::sync::Arc<PostgresPrivacyRepository>,
    pub badge_repository: std::sync::Arc<PostgresBadgeRepository>,
    pub nats_core: async_nats::Client,
    pub presence_preference_subject: String,
    pub identity_users: Arc<IdentityUsersClient>,
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
    pub send_community_invite_usecase: SendCommunityInvite<
        PostgresDmMessageRepository,
        PostgresFriendshipRepository,
        PostgresCommunityRepository,
        PostgresCommunityUnitOfWork,
        PostgresPrivacyRepository,
        PostgresUserProfileRepository,
    >,
    pub accept_community_invite_usecase:
        AcceptCommunityInvite<PostgresDmMessageRepository, PostgresCommunityUnitOfWork>,
    pub decline_community_invite_usecase:
        DeclineCommunityInvite<PostgresDmMessageRepository, PostgresDmUnitOfWork>,
    pub list_sent_community_invites_usecase:
        ListSentCommunityInvites<PostgresDmMessageRepository, PostgresCommunityRepository>,
    pub create_community_usecase: CreateCommunity<PostgresCommunityUnitOfWork, TextModerationClient>,
    pub delete_community_usecase: DeleteCommunity<
        PostgresCommunityRepository,
        PostgresChannelRepository,
        PostgresCommunityUnitOfWork,
        LiveKitService,
    >,
    pub update_community_usecase: UpdateCommunity<PostgresCommunityUnitOfWork, TextModerationClient>,
    pub create_invite_link_usecase: CreateInviteLink<PostgresCommunityUnitOfWork>,
    pub list_invite_links_usecase:
        ListInviteLinks<PostgresCommunityRepository, PostgresCommunityUnitOfWork>,
    pub revoke_invite_link_usecase: RevokeInviteLink<PostgresCommunityUnitOfWork>,
    pub preview_invite_link_usecase: PreviewInviteLink<PostgresCommunityRepository>,
    pub join_by_invite_link_usecase:
        JoinByInviteLink<PostgresCommunityRepository, PostgresCommunityUnitOfWork>,
    pub public_app_url: String,
    pub leave_community_usecase:
        LeaveCommunity<PostgresCommunityUnitOfWork>,
    pub kick_member_usecase:
        KickMember<PostgresCommunityUnitOfWork>,
    pub list_communities_by_user_usecase: ListCommunitiesByUser<PostgresCommunityRepository>,
    pub list_community_members_usecase: ListCommunityMembers<PostgresCommunityRepository>,
    pub list_community_channels_usecase: ListCommunityChannels<PostgresCommunityRepository>,
    pub get_home_overview_usecase: GetHomeOverview<PostgresChannelRepository>,
    pub get_user_profile_usecase:
        GetUserProfile<PostgresUserProfileRepository, PostgresCommunityRepository, PostgresChannelRepository>,
    pub get_voice_presence_usecase:
        GetVoicePresence<PostgresCommunityRepository, PostgresChannelRepository>,
    pub update_stream_viewer_usecase:
        UpdateStreamViewer<PostgresCommunityRepository, PostgresChannelRepository>,
}
