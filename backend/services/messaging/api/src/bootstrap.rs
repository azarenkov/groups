use std::{error::Error, sync::Arc};

use actix_cors::Cors;
use actix_web::{App, HttpServer, middleware::Logger, web};
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
        invite_codes::{CreateInviteCode, ListInviteCodes, RedeemInviteCode, RevokeInviteCode},
        kick_member::KickMember, leave_server::LeaveServer,
        list_sent_server_invites::ListSentServerInvites,
        list_server_channels::ListServerChannels, list_server_members::ListServerMembers,
        list_servers_by_user::ListServersByUser, send_server_invite::SendServerInvite,
        update_server::UpdateServer,
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
        connection::{create_postgres_pool, ping_pool},
        repositories::{
            channel::PostgresChannelRepository,
            dm_message::PostgresDmMessageRepository,
            friendship::PostgresFriendshipRepository, message::PostgresMessageRepository,
            notification_preference::PostgresNotificationPreferenceRepository,
            read_state::PostgresReadStateRepository, server::PostgresServerRepository,
            server_role::PostgresServerRoleRepository,
            channel_unit_of_work::PostgresChannelUnitOfWork,
            dm_unit_of_work::PostgresDmUnitOfWork,
            friendship_unit_of_work::PostgresFriendshipUnitOfWork,
            message_unit_of_work::PostgresMessageUnitOfWork,
            server_unit_of_work::PostgresServerUnitOfWork,
            privacy::PostgresPrivacyRepository, user_profile::PostgresUserProfileRepository,
        },
    },
    media_client::claims::MediaClaimsClient,
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
    let postgres_config = PostgresConfig::from_env()?;
    let livekit_config = LiveKitConfig::from_env()?;
    let rate_limiting_config = RateLimitingConfig::from_env()?;
    let minio_config = MinioConfig::from_env()?;
    let nats_config = infrastructure_common::config::nats::NatsConfig::from_env()?;

    let postgres_pool = create_postgres_pool(postgres_config).await?;
    let nats_jetstream =
        infrastructure_common::messaging::nats::connection::create_jetstream(&nats_config).await?;
    let livekit_url = livekit_config.url().to_string();

    let channel_repository = Arc::new(PostgresChannelRepository::new(postgres_pool.clone()));
    let message_repository = Arc::new(PostgresMessageRepository::new(postgres_pool.clone()));
    let user_repository = Arc::new(PostgresUserProfileRepository::messaging(postgres_pool.clone()));
    let server_repository = Arc::new(PostgresServerRepository::new(postgres_pool.clone()));
    let friendship_repository =
        Arc::new(PostgresFriendshipRepository::new(postgres_pool.clone()));
    let read_state_repository =
        Arc::new(PostgresReadStateRepository::new(postgres_pool.clone()));
    let server_role_repository =
        Arc::new(PostgresServerRoleRepository::new(postgres_pool.clone()));
    let server_unit_of_work = Arc::new(PostgresServerUnitOfWork::new(postgres_pool.clone()));
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
    let identity_internal_url = std::env::var("IDENTITY_INTERNAL_URL")
        .unwrap_or_else(|_| "http://identity-service:8080".to_string());
    let mention_resolver = Arc::new(IdentityUsersClient::new(&identity_internal_url));

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
        server_repository.clone(),
        livekit_service.clone(),
    );
    let server_stream = std::env::var("MESSAGING_SERVER_STREAM")
        .unwrap_or_else(|_| "RIFT_SERVER_EVENTS".to_string());
    messaging_infrastructure::consumers::voice_effects::spawn(
        nats_jetstream.clone(),
        messaging_infrastructure::consumers::voice_effects::VoiceEffectsSpec {
            subject_prefix: nats_config.subject_prefix.clone(),
            channel_stream,
            server_stream,
            user_stream: std::env::var("MESSAGING_USER_STREAM")
                .unwrap_or_else(|_| "RIFT_USER_EVENTS".to_string()),
        },
        channel_repository.clone(),
        server_unit_of_work.clone(),
        livekit_service.clone(),
        user_repository.clone(),
    );

    let user_stream = std::env::var("MESSAGING_USER_STREAM")
        .unwrap_or_else(|_| "RIFT_USER_EVENTS".to_string());
    api_common::user_projection::spawn(
        nats_jetstream.clone(),
        nats_config.subject_prefix.clone(),
        user_stream,
        "messaging-user-profiles",
        user_repository.clone(),
    );

    let nats_core = nats_jetstream.client().clone();
    let presence_preference_subject = format!(
        "{}.realtime.presence.preference",
        nats_config.subject_prefix.trim_end_matches('.'),
    );

    let app_state = web::Data::new(AppState {
        create_channel_usecase: CreateChannel::new(
            hasher.clone(),
            server_repository.clone(),
            server_unit_of_work.clone(),
        ),
        join_voice_channel_usecase: JoinVoiceChannel::new(
            livekit_service.clone(),
            channel_repository.clone(),
            user_repository.clone(),
            hasher.clone(),
            server_repository.clone(),
        ),
        mute_channel_usecase: MuteChannel::new(
            channel_repository.clone(),
            user_repository.clone(),
            server_unit_of_work.clone(),
        ),
        unmute_channel_usecase: UnmuteChannel::new(
            channel_repository.clone(),
            user_repository.clone(),
            server_unit_of_work.clone(),
        ),
        delete_channel_usecase: DeleteChannel::new(
            channel_repository.clone(),
            livekit_service.clone(),
            server_unit_of_work.clone(),
        ),
        list_channel_participants_usecase: ListChannelParticipants::new(
            channel_repository.clone(),
            server_repository.clone(),
        ),
        join_text_channel_usecase: JoinTextChannel::new(
            channel_repository.clone(),
            server_repository.clone(),
            channel_unit_of_work.clone(),
            hasher.clone(),
        ),
        send_channel_message_usecase: SendChannelMessage::new(
            message_repository.clone(),
            channel_repository.clone(),
            server_repository.clone(),
            message_unit_of_work.clone(),
            attachment_claims.clone(),
            mention_resolver.clone(),
        ),
        edit_channel_message_usecase: EditChannelMessage::new(
            message_repository.clone(),
            channel_repository.clone(),
            server_repository.clone(),
            message_unit_of_work.clone(),
            attachment_claims.clone(),
            mention_resolver.clone(),
        ),
        delete_channel_message_usecase: DeleteChannelMessage::new(
            message_repository.clone(),
            channel_repository.clone(),
            server_unit_of_work.clone(),
        ),
        list_channel_messages_usecase: ListChannelMessages::new(
            message_repository.clone(),
            channel_repository.clone(),
            server_repository.clone(),
        ),
        search_channel_messages_usecase: SearchChannelMessages::new(
            message_repository.clone(),
            channel_repository.clone(),
            server_repository.clone(),
        ),
        list_message_attachments_usecase: ListMessageAttachments::new(
            message_repository.clone(),
            dm_repository.clone(),
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
            server_repository.clone(),
            server_role_repository.clone(),
        ),
        can_manage_server_usecase: CanManageServer::new(
            server_repository.clone(),
            server_role_repository.clone(),
        ),
        create_role_usecase: CreateRole::new(server_unit_of_work.clone()),
        update_role_usecase: UpdateRole::new(server_unit_of_work.clone()),
        delete_role_usecase: DeleteRole::new(server_unit_of_work.clone()),
        assign_role_usecase: AssignRole::new(server_unit_of_work.clone()),
        revoke_role_usecase: RevokeRole::new(server_unit_of_work.clone()),
        swap_roles_usecase: SwapRoles::new(server_unit_of_work.clone()),
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
            friendship_repository.clone(),
            privacy_repository.clone(),
        ),
        server_repository: server_repository.clone(),
        message_repository: message_repository.clone(),
        friendship_repository: friendship_repository.clone(),
        privacy_repository: privacy_repository.clone(),
        nats_core,
        presence_preference_subject,
        send_dm_message_usecase: SendDmMessage::new(
            dm_repository.clone(),
            dm_unit_of_work.clone(),
            friendship_repository.clone(),
            attachment_claims.clone(),
            privacy_repository.clone(),
        ),
        edit_dm_message_usecase: EditDmMessage::new(
            dm_repository.clone(),
            dm_unit_of_work.clone(),
            attachment_claims.clone(),
        ),
        delete_dm_message_usecase: DeleteDmMessage::new(
            dm_repository.clone(),
            dm_unit_of_work.clone(),
        ),
        list_dm_previews_usecase: ListDmPreviews::new(dm_repository.clone()),
        list_dm_messages_usecase: ListDmMessages::new(dm_repository.clone()),
        search_dm_messages_usecase: SearchDmMessages::new(dm_repository.clone()),
        send_server_invite_usecase: SendServerInvite::new(
            dm_repository.clone(),
            friendship_repository.clone(),
            server_repository.clone(),
            server_unit_of_work.clone(),
            privacy_repository.clone(),
        ),
        accept_server_invite_usecase: AcceptServerInvite::new(
            dm_repository.clone(),
            server_unit_of_work.clone(),
        ),
        decline_server_invite_usecase: DeclineServerInvite::new(
            dm_repository.clone(),
            dm_unit_of_work.clone(),
        ),
        list_sent_server_invites_usecase: ListSentServerInvites::new(
            dm_repository.clone(),
            server_repository.clone(),
        ),
        create_server_usecase: CreateServer::new(server_unit_of_work.clone()),
        delete_server_usecase: DeleteServer::new(
            server_repository.clone(),
            channel_repository.clone(),
            server_unit_of_work.clone(),
            livekit_service.clone(),
        ),
        update_server_usecase: UpdateServer::new(server_unit_of_work.clone()),
        create_invite_code_usecase: CreateInviteCode::new(server_unit_of_work.clone()),
        list_invite_codes_usecase: ListInviteCodes::new(
            server_repository.clone(),
            server_unit_of_work.clone(),
        ),
        revoke_invite_code_usecase: RevokeInviteCode::new(server_unit_of_work.clone()),
        redeem_invite_code_usecase: RedeemInviteCode::new(server_repository.clone()),
        leave_server_usecase: LeaveServer::new(server_unit_of_work.clone()),
        kick_member_usecase: KickMember::new(server_unit_of_work.clone()),
        list_servers_by_user_usecase: ListServersByUser::new(server_repository.clone()),
        list_server_members_usecase: ListServerMembers::new(server_repository.clone()),
        list_server_channels_usecase: ListServerChannels::new(server_repository.clone()),
        get_home_overview_usecase: GetHomeOverview::new(channel_repository.clone()),
        get_user_profile_usecase: GetUserProfile::new(
            user_repository.clone(),
            server_repository.clone(),
            channel_repository.clone(),
        ),
        get_voice_presence_usecase: GetVoicePresence::new(
            server_repository.clone(),
            channel_repository.clone(),
        ),
        update_stream_viewer_usecase: UpdateStreamViewer::new(
            server_repository.clone(),
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
    let readiness_data = web::Data::new(readiness);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .expose_headers(["x-request-id"])
            .max_age(3600);

        App::new()
            .app_data(app_state.clone())
            .app_data(readiness_data.clone())
            .app_data(rate_limiter_data.clone())
            .app_data(access_revocations_data.clone())
            .wrap(RequestIdMiddleware)
            .wrap(Logger::default())
            .wrap(cors)
            .configure(routes::configure)
    })
    .bind((server_config.host, server_config.port))?
    .run()
    .await?;

    Ok(())
}
