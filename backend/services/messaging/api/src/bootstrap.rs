use messaging::usecases::block::{block_user::BlockUser, list_blocked_users::ListBlockedUsers, unblock_user::UnblockUser};
use messaging::usecases::pin::{conversation_pins::ConversationPins, message_pins::MessagePins};
use messaging::usecases::dm_conversation::DmConversations;
use messaging_infrastructure::database::postgres::repositories::dm_conversation::PostgresDmConversationRepository;
use messaging::usecases::search::global_search::GlobalSearch;
use messaging::usecases::shared::list_shared_content::ListSharedContent;
use messaging::usecases::attachment::get_attachment_thumbnail::GetAttachmentThumbnail;
use messaging_infrastructure::thumbnails::ImageThumbnailer;
use std::{error::Error, sync::Arc};
use api_common::shutdown::{self, ShutdownConfig};

use actix_web::{App, HttpServer, middleware::Logger, web};
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
    hidden_message::hide_message::HideMessage,
    voice_listen::voice_listens::VoiceListens,
    channel::disconnect_voice_member::DisconnectVoiceMember,
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
use infrastructure_common::{
    config::{livekit::LiveKitConfig, minio::MinioConfig, postgres::PostgresConfig},
    hasher::bcrypt_hasher::BcryptHasher,
    storage::minio_storage::MinioStorage,
    voice::livekit_service::{LiveKitService, is_livekit_reachable},
};
use messaging_infrastructure::{
    database::postgres::{
        MIGRATOR, SCHEMA,
        connection::{create_postgres_pool, ping_pool},
        repositories::{
            channel::PostgresChannelRepository,
            dm_message::PostgresDmMessageRepository,
            friendship::PostgresFriendshipRepository, message::PostgresMessageRepository,
            notification_preference::PostgresNotificationPreferenceRepository,
            read_state::PostgresReadStateRepository, pin::{PostgresConversationPinRepository, PostgresMessagePinRepository}, search::PostgresSearchRepository, shared::PostgresSharedContentRepository, community::PostgresCommunityRepository,
            hidden_message::PostgresHiddenMessageRepository,
            voice_listen::PostgresVoiceListenRepository,
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
    },
    media_client::claims::MediaClaimsClient,
    media_client::voice_calls::VoiceCallsClient,
    media_client::identity_users::IdentityUsersClient,
};
use api_common::{
    health::Readiness,
    middleware::request_id::RequestIdMiddleware,
    rate_limiting::config::RateLimitingConfig,
};

use api_common::config::ServerConfig;

use crate::{routes, state::AppState};

pub async fn run() -> Result<(), Box<dyn Error>> {
    infrastructure_common::observability::init("messaging-service");

    let server_config = ServerConfig::from_env()?;
    let postgres_config = PostgresConfig::from_env_for(SCHEMA)?;
    let livekit_config = LiveKitConfig::from_env()?;
    let rate_limiting_config = RateLimitingConfig::from_env()?;
    let minio_config = MinioConfig::from_env()?;
    let nats_config = infrastructure_common::config::nats::NatsConfig::from_env()?;
    let nsfw_config = infrastructure_common::config::nsfw::NsfwConfig::from_env()?;
    let text_moderation = Arc::new(
        infrastructure_common::moderation::text_detector::TextModerationClient::new(&nsfw_config),
    );

    let postgres_pool = create_postgres_pool(postgres_config, SCHEMA, Some(&MIGRATOR)).await?;
    let nats_jetstream =
        infrastructure_common::messaging::nats::connection::create_jetstream(&nats_config).await?;
    let livekit_url = livekit_config.url().to_string();

    let channel_repository = Arc::new(PostgresChannelRepository::new(postgres_pool.clone()));
    let message_repository = Arc::new(PostgresMessageRepository::new(postgres_pool.clone()));
    let user_repository = Arc::new(PostgresUserProfileRepository::messaging(postgres_pool.clone()));
    let community_repository = Arc::new(PostgresCommunityRepository::new(postgres_pool.clone()));
    let friendship_repository =
        Arc::new(PostgresFriendshipRepository::new(postgres_pool.clone()));
    let read_state_repository =
        Arc::new(PostgresReadStateRepository::new(postgres_pool.clone()));
    let community_role_repository =
        Arc::new(PostgresCommunityRoleRepository::new(postgres_pool.clone()));
    let community_unit_of_work = Arc::new(PostgresCommunityUnitOfWork::new(postgres_pool.clone()));
    let message_unit_of_work = Arc::new(PostgresMessageUnitOfWork::new(postgres_pool.clone()));
    let dm_unit_of_work = Arc::new(PostgresDmUnitOfWork::new(postgres_pool.clone()));
    let friendship_unit_of_work =
        Arc::new(PostgresFriendshipUnitOfWork::new(postgres_pool.clone()));
    let channel_unit_of_work = Arc::new(PostgresChannelUnitOfWork::new(postgres_pool.clone()));
    let notification_preference_repository = Arc::new(
        PostgresNotificationPreferenceRepository::new(postgres_pool.clone()),
    );
    let dm_repository = Arc::new(PostgresDmMessageRepository::new(postgres_pool.clone()));
    let media_internal_url = std::env::var("MEDIA_INTERNAL_URL")
        .unwrap_or_else(|_| "http://media-service:8080".to_string());
    let attachment_claims = Arc::new(MediaClaimsClient::new(&media_internal_url));
    let privacy_repository = Arc::new(PostgresPrivacyRepository::new(postgres_pool.clone()));
    let badge_repository = Arc::new(PostgresBadgeRepository::new(postgres_pool.clone()));
    let plan_directory = Arc::new(
        infrastructure_common::database::postgres::repositories::plan_directory::PostgresPlanDirectory::messaging(
            postgres_pool.clone(),
        ),
    );
    let report_repository = Arc::new(PostgresReportRepository::new(postgres_pool.clone()));
    let identity_internal_url = std::env::var("IDENTITY_INTERNAL_URL")
        .unwrap_or_else(|_| "http://identity-service:8080".to_string());
    let mention_resolver = Arc::new(IdentityUsersClient::new(&identity_internal_url));
    let voice_internal_url = std::env::var("VOICE_INTERNAL_URL")
        .unwrap_or_else(|_| "http://voice-service:8080".to_string());
    let call_participation = Arc::new(VoiceCallsClient::new(&voice_internal_url));

    let minio_storage = Arc::new(
        MinioStorage::new(&minio_config)
            .map_err(|e| Box::<dyn Error>::from(format!("minio init failed: {e}")))?,
    );
    let hasher = Arc::new(BcryptHasher::default());
    let livekit_service = Arc::new(LiveKitService::new(livekit_config));

    let channel_stream = std::env::var("MESSAGING_CHANNEL_STREAM")
        .unwrap_or_else(|_| "RIFT_CHANNEL_EVENTS".to_string());
    messaging_infrastructure::consumers::voice_webhooks::spawn(
        nats_jetstream.clone(),
        nats_config.subject_prefix.clone(),
        channel_stream.clone(),
        channel_repository.clone(),
        channel_unit_of_work.clone(),
        community_repository.clone(),
        livekit_service.clone(),
    );
    let community_stream = std::env::var("MESSAGING_COMMUNITY_STREAM")
        .unwrap_or_else(|_| "RIFT_COMMUNITY_EVENTS".to_string());
    messaging_infrastructure::consumers::voice_effects::spawn(
        nats_jetstream.clone(),
        messaging_infrastructure::consumers::voice_effects::VoiceEffectsSpec {
            subject_prefix: nats_config.subject_prefix.clone(),
            channel_stream,
            community_stream,
            user_stream: std::env::var("MESSAGING_USER_STREAM")
                .unwrap_or_else(|_| "RIFT_USER_EVENTS".to_string()),
        },
        channel_repository.clone(),
        community_unit_of_work.clone(),
        livekit_service.clone(),
        user_repository.clone(),
    );

    let user_stream = std::env::var("MESSAGING_USER_STREAM")
        .unwrap_or_else(|_| "RIFT_USER_EVENTS".to_string());
    api_common::user_projection::spawn(
        nats_jetstream.clone(),
        nats_config.subject_prefix.clone(),
        user_stream.clone(),
        "messaging-user-profiles",
        user_repository.clone(),
    );
    infrastructure_common::messaging::nats::plan_projection::PlanProjection {
        jetstream: nats_jetstream.clone(),
        subject_prefix: nats_config.subject_prefix.clone(),
        stream: user_stream.clone(),
        consumer_name: "messaging-projection-tiers",
        dlq_suffix: "messaging.tiers",
        plans: plan_directory.clone(),
        listener: Arc::new(ports::plan_directory::NoPlanListener),
        caught_up: Arc::new(std::sync::atomic::AtomicBool::new(false)),
    }
    .spawn();
    messaging_infrastructure::consumers::pro_badges::spawn(
        nats_jetstream.clone(),
        nats_config.subject_prefix.clone(),
        user_stream,
        badge_repository.clone(),
    );

    let nats_core = nats_jetstream.client().clone();
    let presence_preference_subject = format!(
        "{}.realtime.presence.preference",
        nats_config.subject_prefix.trim_end_matches('.'),
    );

    let public_app_url = std::env::var("PUBLIC_APP_URL")
        .ok()
        .map(|url| url.trim().trim_end_matches('/').to_string())
        .filter(|url| url.starts_with("https://") || url.starts_with("http://"))
        .ok_or("PUBLIC_APP_URL must be set to the public web app origin (http(s)://host), invite links are built from it")?;

    let app_state = web::Data::new(AppState {
        create_channel_usecase: CreateChannel::new(
            hasher.clone(),
            community_repository.clone(),
            community_unit_of_work.clone(),
        ),
        join_voice_channel_usecase: JoinVoiceChannel::new(
            livekit_service.clone(),
            channel_repository.clone(),
            user_repository.clone(),
            hasher.clone(),
            community_repository.clone(),
        ),
        disconnect_voice_member_usecase: DisconnectVoiceMember::new(
            channel_repository.clone(),
            community_unit_of_work.clone(),
            livekit_service.clone(),
        ),
        mute_channel_usecase: MuteChannel::new(
            channel_repository.clone(),
            user_repository.clone(),
            community_unit_of_work.clone(),
        ),
        unmute_channel_usecase: UnmuteChannel::new(
            channel_repository.clone(),
            user_repository.clone(),
            community_unit_of_work.clone(),
        ),
        deafen_channel_usecase: DeafenChannel::new(
            channel_repository.clone(),
            user_repository.clone(),
            community_unit_of_work.clone(),
        ),
        undeafen_channel_usecase: UndeafenChannel::new(
            channel_repository.clone(),
            user_repository.clone(),
            community_unit_of_work.clone(),
        ),
        delete_channel_usecase: DeleteChannel::new(
            channel_repository.clone(),
            livekit_service.clone(),
            community_unit_of_work.clone(),
        ),
        list_channel_participants_usecase: ListChannelParticipants::new(
            channel_repository.clone(),
            community_repository.clone(),
            friendship_repository.clone(),
        ),
        join_text_channel_usecase: JoinTextChannel::new(
            channel_repository.clone(),
            community_repository.clone(),
            channel_unit_of_work.clone(),
            hasher.clone(),
        ),
        send_channel_message_usecase: SendChannelMessage::new(
            message_repository.clone(),
            channel_repository.clone(),
            community_repository.clone(),
            message_unit_of_work.clone(),
            attachment_claims.clone(),
            mention_resolver.clone(),
            plan_directory.clone(),
        ),
        edit_channel_message_usecase: EditChannelMessage::new(
            message_repository.clone(),
            channel_repository.clone(),
            community_repository.clone(),
            message_unit_of_work.clone(),
            attachment_claims.clone(),
            mention_resolver.clone(),
            plan_directory.clone(),
        ),
        delete_channel_message_usecase: DeleteChannelMessage::new(
            message_repository.clone(),
            channel_repository.clone(),
            community_unit_of_work.clone(),
        ),
        list_channel_messages_usecase: ListChannelMessages::new(
            message_repository.clone(),
            channel_repository.clone(),
            community_repository.clone(),
        ),
        search_channel_messages_usecase: SearchChannelMessages::new(
            message_repository.clone(),
            channel_repository.clone(),
            community_repository.clone(),
        ),
        global_search_usecase: GlobalSearch::new(Arc::new(PostgresSearchRepository::new(postgres_pool.clone()))),
        message_pins_usecase: MessagePins::new(
            Arc::new(PostgresMessagePinRepository::new(postgres_pool.clone())),
            channel_repository.clone(),
            community_repository.clone(),
            community_unit_of_work.clone(),
            friendship_repository.clone(),
        ),
        conversation_pins_usecase: ConversationPins::new(Arc::new(PostgresConversationPinRepository::new(
            postgres_pool.clone(),
        ))),
        partition_sequences: infrastructure_common::database::postgres::repositories::outbox_record::PostgresPartitionSequences::new(postgres_pool.clone()),
        dm_conversations_usecase: DmConversations::new(Arc::new(PostgresDmConversationRepository::new(
            postgres_pool.clone(),
        ))),
        list_shared_content_usecase: ListSharedContent::new(
            Arc::new(PostgresSharedContentRepository::new(postgres_pool.clone())),
            channel_repository.clone(),
            community_repository.clone(),
        ),
        list_message_attachments_usecase: ListMessageAttachments::new(
            message_repository.clone(),
            dm_repository.clone(),
        ),
        get_attachment_thumbnail_usecase: GetAttachmentThumbnail::new(
            message_repository.clone(),
            minio_storage.clone(),
            Arc::new(ImageThumbnailer),
            Arc::new(messaging_infrastructure::outbox::storage::PostgresStorageReleases::new(postgres_pool.clone())),
        ),
        get_attachment_usecase: GetAttachment::new(
            message_repository.clone(),
            minio_storage.clone(),
            attachment_claims.clone(),
        ),
        send_friend_request_usecase: SendFriendRequest::new(
            friendship_unit_of_work.clone(),
            user_repository.clone(),
            privacy_repository.clone(),
        ),
        get_privacy_usecase: GetPrivacy::new(privacy_repository.clone()),
        update_privacy_usecase: UpdatePrivacy::new(privacy_repository.clone()),
        accept_friend_request_usecase: AcceptFriendRequest::new(friendship_unit_of_work.clone()),
        remove_friendship_usecase: RemoveFriendship::new(friendship_unit_of_work.clone()),
        list_friends_usecase: ListFriends::new(friendship_repository.clone()),
        create_report_usecase: CreateReport::new(
            report_repository.clone(),
            user_repository.clone(),
            community_repository.clone(),
            message_repository.clone(),
            dm_repository.clone(),
            channel_repository.clone(),
            attachment_claims.clone(),
        ),
        block_user_usecase: BlockUser::new(friendship_unit_of_work.clone(), user_repository.clone()),
        unblock_user_usecase: UnblockUser::new(friendship_unit_of_work.clone()),
        list_blocked_users_usecase: ListBlockedUsers::new(friendship_repository.clone()),
        hide_message_usecase: HideMessage::new(Arc::new(PostgresHiddenMessageRepository::new(
            postgres_pool.clone(),
        ))),
        voice_listens_usecase: VoiceListens::new(Arc::new(PostgresVoiceListenRepository::new(
            postgres_pool.clone(),
        ))),
        mark_channel_read_usecase: MarkChannelRead::new(
            read_state_repository.clone(),
            channel_repository.clone(),
        ),
        mark_dm_read_usecase: MarkDmRead::new(
            read_state_repository.clone(),
            friendship_repository.clone(),
            dm_repository.clone(),
        ),
        get_unread_counts_usecase: GetUnreadCounts::new(read_state_repository.clone()),
        get_dm_read_receipt_usecase: GetDmReadReceipt::new(
            read_state_repository.clone(),
            friendship_repository.clone(),
            dm_repository.clone(),
        ),
        get_channel_read_receipts_usecase: GetChannelReadReceipts::new(
            read_state_repository.clone(),
            channel_repository.clone(),
        ),
        list_roles_usecase: ListRoles::new(
            community_repository.clone(),
            community_role_repository.clone(),
        ),
        can_manage_community_usecase: CanManageCommunity::new(
            community_repository.clone(),
            community_role_repository.clone(),
        ),
        create_role_usecase: CreateRole::new(community_unit_of_work.clone()),
        update_role_usecase: UpdateRole::new(community_unit_of_work.clone()),
        delete_role_usecase: DeleteRole::new(community_unit_of_work.clone()),
        assign_role_usecase: AssignRole::new(community_unit_of_work.clone()),
        revoke_role_usecase: RevokeRole::new(community_unit_of_work.clone()),
        swap_roles_usecase: SwapRoles::new(community_unit_of_work.clone()),
        list_notification_preferences_usecase: ListNotificationPreferences::new(
            notification_preference_repository.clone(),
        ),
        set_notification_preference_usecase: SetNotificationPreference::new(
            notification_preference_repository.clone(),
            channel_repository.clone(),
            friendship_repository.clone(),
        ),
        log_call_usecase: LogCall::new(
            dm_unit_of_work.clone(),
            call_participation.clone(),
        ),
        community_repository: community_repository.clone(),
        message_repository: message_repository.clone(),
        dm_repository: dm_repository.clone(),
        friendship_repository: friendship_repository.clone(),
        privacy_repository: privacy_repository.clone(),
        badge_repository: badge_repository.clone(),
        nats_core,
        presence_preference_subject,
        identity_users: mention_resolver.clone(),
        send_dm_message_usecase: SendDmMessage::new(
            dm_repository.clone(),
            dm_unit_of_work.clone(),
            friendship_repository.clone(),
            attachment_claims.clone(),
            privacy_repository.clone(),
            user_repository.clone(),
            plan_directory.clone(),
        ),
        edit_dm_message_usecase: EditDmMessage::new(
            dm_repository.clone(),
            dm_unit_of_work.clone(),
            attachment_claims.clone(),
            friendship_repository.clone(),
            privacy_repository.clone(),
            user_repository.clone(),
            plan_directory.clone(),
        ),
        delete_dm_message_usecase: DeleteDmMessage::new(
            dm_repository.clone(),
            dm_unit_of_work.clone(),
        ),
        list_dm_previews_usecase: ListDmPreviews::new(dm_repository.clone()),
        list_dm_messages_usecase: ListDmMessages::new(dm_repository.clone()),
        search_dm_messages_usecase: SearchDmMessages::new(dm_repository.clone()),
        send_community_invite_usecase: SendCommunityInvite::new(
            dm_repository.clone(),
            friendship_repository.clone(),
            community_repository.clone(),
            community_unit_of_work.clone(),
            privacy_repository.clone(),
            user_repository.clone(),
        ),
        accept_community_invite_usecase: AcceptCommunityInvite::new(
            dm_repository.clone(),
            community_unit_of_work.clone(),
        ),
        decline_community_invite_usecase: DeclineCommunityInvite::new(
            dm_repository.clone(),
            dm_unit_of_work.clone(),
        ),
        list_sent_community_invites_usecase: ListSentCommunityInvites::new(
            dm_repository.clone(),
            community_repository.clone(),
        ),
        create_community_usecase: CreateCommunity::new(community_unit_of_work.clone(), text_moderation.clone()),
        delete_community_usecase: DeleteCommunity::new(
            community_repository.clone(),
            channel_repository.clone(),
            community_unit_of_work.clone(),
            livekit_service.clone(),
        ),
        update_community_usecase: UpdateCommunity::new(community_unit_of_work.clone(), text_moderation.clone()),
        create_invite_link_usecase: CreateInviteLink::new(community_unit_of_work.clone()),
        list_invite_links_usecase: ListInviteLinks::new(
            community_repository.clone(),
            community_unit_of_work.clone(),
        ),
        revoke_invite_link_usecase: RevokeInviteLink::new(community_unit_of_work.clone()),
        preview_invite_link_usecase: PreviewInviteLink::new(community_repository.clone()),
        join_by_invite_link_usecase: JoinByInviteLink::new(
            community_repository.clone(),
            community_unit_of_work.clone(),
        ),
        public_app_url: public_app_url.clone(),
        leave_community_usecase: LeaveCommunity::new(community_unit_of_work.clone()),
        kick_member_usecase: KickMember::new(community_unit_of_work.clone()),
        list_communities_by_user_usecase: ListCommunitiesByUser::new(community_repository.clone()),
        list_community_members_usecase: ListCommunityMembers::new(community_repository.clone()),
        list_community_channels_usecase: ListCommunityChannels::new(community_repository.clone()),
        get_home_overview_usecase: GetHomeOverview::new(channel_repository.clone()),
        get_user_profile_usecase: GetUserProfile::new(
            user_repository.clone(),
            community_repository.clone(),
            channel_repository.clone(),
        ),
        get_voice_presence_usecase: GetVoicePresence::new(
            community_repository.clone(),
            channel_repository.clone(),
        ),
        update_stream_viewer_usecase: UpdateStreamViewer::new(
            community_repository.clone(),
            channel_repository.clone(),
        ),
    });

    let rate_limiter =
        api_common::rate_limiting::create_rate_limiter_from_config("api", &rate_limiting_config)
            .await
            .ok_or("Failed to create rate limiter: invalid parameters")?;
    let rate_limiter_data = web::Data::new(rate_limiter);
    let access_revocations_data =
        web::Data::new(api_common::middleware::revocation::AccessRevocations::from_env().await);

    let readiness = Readiness::builder()
        .add_probe({
            let pool = postgres_pool.clone();
            move || {
                let pool = pool.clone();
                async move { ping_pool(&pool).await }
            }
        })
        .add_probe({
            let url = livekit_url.clone();
            move || {
                let url = url.clone();
                async move { is_livekit_reachable(&url).await }
            }
        })
        .build();
    readiness.mark_ready();
    let readiness_data = web::Data::new(readiness.clone());
    let shutdown_config = ShutdownConfig::from_env();

    let server = HttpServer::new(move || {
        let cors = api_common::cors::cors();

        App::new()
            .wrap(actix_web::middleware::from_fn(
                api_common::middleware::internal::guard_internal_routes,
            ))
            .app_data(app_state.clone())
            .app_data(readiness_data.clone())
            .app_data(rate_limiter_data.clone())
            .app_data(access_revocations_data.clone())
            .wrap(RequestIdMiddleware)
            .wrap(Logger::default().exclude("/readyz").exclude("/livez"))
            .wrap(cors)
            .configure(routes::configure)
    })
    .disable_signals()
    .shutdown_timeout(shutdown_config.shutdown_timeout.as_secs())
    .bind((server_config.host, server_config.port))?
    .run();
    shutdown::serve(server, readiness, shutdown_config).await?;

    Ok(())
}
