use std::{error::Error, sync::Arc};

use actix_cors::Cors;
use actix_web::{App, HttpServer, middleware::Logger, web};
use application::usecases::{
    attachment::{
        cleanup_orphans::CleanupOrphanAttachments, delete_attachment::DeleteAttachment,
        get_attachment::GetAttachment, list_message_attachments::ListMessageAttachments,
        upload_attachment::UploadAttachment,
    },
    auth::{
        confirm_registration::ConfirmRegistration, google_login::GoogleLogin,
        issue_verification_code::IssueVerificationCode, login::LoginUser, logout::Logout,
        refresh_tokens::RefreshTokens, register::RegisterUser,
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
    user::{
        delete_avatar::DeleteUserAvatar, get_avatar::GetUserAvatar, get_me::GetMe,
        search_users::SearchUsers, update_avatar::UpdateUserAvatar,
        update_locale::UpdateUserLocale,
    },
    call::{
        accept_call::AcceptCall, add_participants::AddCallParticipants,
        decline_call::DeclineCall, get_active_call::GetActiveCall,
        get_active_call_with::GetActiveCallWith, join_call::JoinCall, leave_call::LeaveCall,
        log_call::LogCall, start_call::StartCall,
    },
    channel::{
        abort_voice_connection::AbortVoiceConnection, create_channel::CreateChannel,
        delete_channel::DeleteChannel, delete_channel_message::DeleteChannelMessage,
        edit_channel_message::EditChannelMessage, join_text_channel::JoinTextChannel,
        join_voice_channel::JoinVoiceChannel, join_voice_connection::JoinVoiceConnection,
        leave_voice_connection::LeaveVoiceConnection,
        list_channel_messages::ListChannelMessages,
        list_channel_participants::ListChannelParticipants, mute_channel::MuteChannel,
        search_channel_messages::SearchChannelMessages,
        send_channel_message::SendChannelMessage,
        track_media_connection::TrackMediaConnection, unmute_channel::UnmuteChannel,
        update_stream_viewer::UpdateStreamViewer,
    },
    server::{
        accept_server_invite::AcceptServerInvite, create_server::CreateServer,
        decline_server_invite::DeclineServerInvite, delete_server::DeleteServer,
        invite_codes::{
            CreateInviteCode, ListInviteCodes, RedeemInviteCode, RevokeInviteCode,
        },
        get_logo::GetServerLogo, update_logo::UpdateServerLogo,
        update_server::UpdateServer,
        leave_server::LeaveServer, list_sent_server_invites::ListSentServerInvites,
        get_voice_presence::GetVoicePresence, list_server_channels::ListServerChannels,
        list_server_members::ListServerMembers,
        list_servers_by_user::ListServersByUser, send_server_invite::SendServerInvite,
    },
    webhook::voice::HandleVoiceWebhook,
};
use infrastructure::{
    auth::{google_verifier::GoogleIdTokenVerifier, jwt_token_issuer::JwtTokenIssuer},
    config::{
        google::GoogleOAuthConfig, livekit::LiveKitConfig, minio::MinioConfig,
        nsfw::NsfwConfig, postgres::PostgresConfig,
    },
    database::postgres::{
        connection::{create_postgres_pool, ping_pool},
        repositories::{
            attachment::PostgresAttachmentRepository,
            call::PostgresCallRepository, channel::PostgresChannelRepository,
            dm_message::PostgresDmMessageRepository,
            friendship::PostgresFriendshipRepository,
            message::PostgresMessageRepository,
            read_state::PostgresReadStateRepository,
            refresh_token::PostgresRefreshTokenRepository, server::PostgresServerRepository,
            user::PostgresUserRepository,
            verification_code::PostgresVerificationCodeRepository,
        },
    },
    hasher::bcrypt_hasher::BcryptHasher,
    moderation::nsfw_detector::NsfwDetector,
    storage::minio_storage::MinioStorage,
    voice::{
        livekit_service::{LiveKitService, is_livekit_reachable},
        livekit_webhook_receiver::LiveKitWebhookReceiver,
    },
};
use shared::{
    health::Readiness,
    rate_limiting::{config::RateLimitingConfig, create_rate_limiter},
};

use shared::middleware::request_id::RequestIdMiddleware;

use crate::{config::ServerConfig, routes, shared::state::AppState};

pub async fn run() -> Result<(), Box<dyn Error>> {
    shared::observability::init("http");

    let server_config = ServerConfig::from_env()?;
    let postgres_config = PostgresConfig::from_env()?;
    let livekit_config = LiveKitConfig::from_env()?;
    let google_config = GoogleOAuthConfig::from_env()?;
    let rate_limiting_config = RateLimitingConfig::from_env()?;
    let minio_config = MinioConfig::from_env()?;
    let nats_config = infrastructure::config::nats::NatsConfig::from_env()?;

    let postgres_pool = create_postgres_pool(postgres_config).await?;
    let nats_jetstream =
        infrastructure::messaging::nats::connection::create_jetstream(&nats_config).await?;
    let livekit_url = livekit_config.url().to_string();

    let channel_repository = Arc::new(PostgresChannelRepository::new(postgres_pool.clone()));
    let call_repository = Arc::new(PostgresCallRepository::new(postgres_pool.clone()));
    let message_repository = Arc::new(PostgresMessageRepository::new(postgres_pool.clone()));
    let user_repository = Arc::new(PostgresUserRepository::new(postgres_pool.clone()));
    let server_repository = Arc::new(PostgresServerRepository::new(postgres_pool.clone()));
    let verification_code_repository = Arc::new(PostgresVerificationCodeRepository::new(
        postgres_pool.clone(),
    ));
    let refresh_token_repository =
        Arc::new(PostgresRefreshTokenRepository::new(postgres_pool.clone()));

    let attachment_repository = Arc::new(PostgresAttachmentRepository::new(postgres_pool.clone()));
    let minio_storage = Arc::new(
        MinioStorage::new(&minio_config)
            .map_err(|e| Box::<dyn Error>::from(format!("minio init failed: {e}")))?,
    );
    if let Err(e) = minio_storage.ensure_bucket(&minio_config).await {
        tracing::warn!(error = %e, "Failed to ensure MinIO bucket; retrying in background");
        let retry_storage = minio_storage.clone();
        let retry_config = minio_config.clone();
        actix_web::rt::spawn(async move {
            let mut delay = std::time::Duration::from_secs(2);
            loop {
                actix_web::rt::time::sleep(delay).await;
                match retry_storage.ensure_bucket(&retry_config).await {
                    Ok(()) => {
                        tracing::info!("MinIO bucket ensured after retry");
                        break;
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "MinIO bucket ensure retry failed");
                        delay = (delay * 2).min(std::time::Duration::from_secs(60));
                    }
                }
            }
        });
    }
    let upload_attachment_usecase =
        UploadAttachment::new(attachment_repository.clone(), minio_storage.clone());
    let upload_semaphore = Arc::new(tokio::sync::Semaphore::new(8));
    let get_attachment_usecase =
        GetAttachment::new(attachment_repository.clone(), minio_storage.clone());
    let delete_attachment_usecase =
        DeleteAttachment::new(attachment_repository.clone(), minio_storage.clone());
    let orphan_cleanup = Arc::new(CleanupOrphanAttachments::new(
        attachment_repository.clone(),
        minio_storage.clone(),
    ));
    {
        let orphan_cleanup = orphan_cleanup.clone();
        actix_web::rt::spawn(async move {
            let mut interval =
                actix_web::rt::time::interval(std::time::Duration::from_secs(600));
            loop {
                interval.tick().await;
                orphan_cleanup.run_once().await;
            }
        });
    }
    let list_message_attachments_usecase =
        ListMessageAttachments::new(attachment_repository.clone());
    let nsfw_config = NsfwConfig::from_env()?;
    let nsfw_detector = Arc::new(NsfwDetector::new(&nsfw_config));
    let update_user_avatar_usecase = UpdateUserAvatar::new(
        user_repository.clone(),
        attachment_repository.clone(),
        minio_storage.clone(),
        nsfw_detector.clone(),
    );
    let delete_user_avatar_usecase = DeleteUserAvatar::new(
        user_repository.clone(),
        attachment_repository.clone(),
        minio_storage.clone(),
    );
    let get_user_avatar_usecase =
        GetUserAvatar::new(user_repository.clone(), minio_storage.clone());
    let update_server_logo_usecase = UpdateServerLogo::new(
        server_repository.clone(),
        attachment_repository.clone(),
        minio_storage.clone(),
        nsfw_detector.clone(),
    );
    let get_server_logo_usecase =
        GetServerLogo::new(server_repository.clone(), minio_storage.clone());
    let hasher = Arc::new(BcryptHasher::default());
    let livekit_webhook_receiver = Arc::new(LiveKitWebhookReceiver::new(&livekit_config));
    let livekit_service = Arc::new(LiveKitService::new(livekit_config));
    let token_issuer = Arc::new(JwtTokenIssuer::new(server_config.jwt_secret.clone()));

    let create_channel_usecase = CreateChannel::new(
        channel_repository.clone(),
        hasher.clone(),
        server_repository.clone(),
    );
    let join_voice_channel_usecase = JoinVoiceChannel::new(
        livekit_service.clone(),
        channel_repository.clone(),
        user_repository.clone(),
        hasher.clone(),
    );
    let mute_channel_usecase =
        MuteChannel::new(channel_repository.clone(), user_repository.clone());
    let unmute_channel_usecase =
        UnmuteChannel::new(channel_repository.clone(), user_repository.clone());
    let delete_channel_usecase =
        DeleteChannel::new(channel_repository.clone(), livekit_service.clone());
    let list_channel_participants_usecase =
        ListChannelParticipants::new(channel_repository.clone(), server_repository.clone());
    let join_text_channel_usecase = JoinTextChannel::new(
        channel_repository.clone(),
        server_repository.clone(),
        hasher.clone(),
    );
    let send_channel_message_usecase = SendChannelMessage::new(
        message_repository.clone(),
        channel_repository.clone(),
        server_repository.clone(),
        attachment_repository.clone(),
    );
    let edit_channel_message_usecase = EditChannelMessage::new(
        message_repository.clone(),
        channel_repository.clone(),
        server_repository.clone(),
        attachment_repository.clone(),
        minio_storage.clone(),
    );
    let delete_channel_message_usecase = DeleteChannelMessage::new(
        message_repository.clone(),
        channel_repository.clone(),
        server_repository.clone(),
        attachment_repository.clone(),
        minio_storage.clone(),
    );
    let list_channel_messages_usecase = ListChannelMessages::new(
        message_repository.clone(),
        channel_repository.clone(),
        server_repository.clone(),
    );
    let search_channel_messages_usecase = SearchChannelMessages::new(
        message_repository.clone(),
        channel_repository.clone(),
        server_repository.clone(),
    );
    let register_user_usecase = RegisterUser::new(
        user_repository.clone(),
        hasher.clone(),
        token_issuer.clone(),
    );
    let issue_verification_code_usecase = IssueVerificationCode::new(
        user_repository.clone(),
        verification_code_repository.clone(),
        hasher.clone(),
    );
    let login_user_usecase = LoginUser::new(
        user_repository.clone(),
        hasher.clone(),
        refresh_token_repository.clone(),
        token_issuer.clone(),
    );
    let google_audiences = google_config.audiences();
    if google_audiences.is_empty() {
        tracing::warn!(
            "GOOGLE_CLIENT_ID is not set; /v1/auth/google will reject requests with GOOGLE_NOT_CONFIGURED"
        );
    }
    let google_token_verifier = Arc::new(GoogleIdTokenVerifier::new(google_audiences));
    let google_login_usecase = GoogleLogin::new(
        user_repository.clone(),
        refresh_token_repository.clone(),
        token_issuer.clone(),
        google_token_verifier.clone(),
        hasher.clone(),
    );
    let confirm_registration_usecase = ConfirmRegistration::new(
        user_repository.clone(),
        hasher.clone(),
        verification_code_repository.clone(),
    );
    let refresh_tokens_usecase = RefreshTokens::new(
        refresh_token_repository.clone(),
        user_repository.clone(),
        token_issuer.clone(),
    );
    let logout_usecase = Logout::new(refresh_token_repository.clone(), token_issuer.clone());

    let join_voice_connection_usecase = JoinVoiceConnection::new(channel_repository.clone());
    let abort_voice_connection_usecase = AbortVoiceConnection::new(channel_repository.clone());
    let leave_voice_connection_usecase = LeaveVoiceConnection::new(channel_repository.clone());
    let track_media_connection_usecase = TrackMediaConnection::new(channel_repository.clone());
    let handle_livekit_webhook_usecase = HandleVoiceWebhook::new(
        livekit_webhook_receiver.clone(),
        join_voice_connection_usecase,
        abort_voice_connection_usecase,
        leave_voice_connection_usecase,
        track_media_connection_usecase,
        call_repository.clone(),
    );

    let create_server_usecase = CreateServer::new(server_repository.clone());
    let update_server_usecase = UpdateServer::new(server_repository.clone());
    let create_invite_code_usecase = CreateInviteCode::new(server_repository.clone());
    let list_invite_codes_usecase = ListInviteCodes::new(server_repository.clone());
    let revoke_invite_code_usecase = RevokeInviteCode::new(server_repository.clone());
    let redeem_invite_code_usecase = RedeemInviteCode::new(server_repository.clone());
    let delete_server_usecase = DeleteServer::new(
        server_repository.clone(),
        channel_repository.clone(),
        livekit_service.clone(),
    );
    let leave_server_usecase = LeaveServer::new(
        server_repository.clone(),
        channel_repository.clone(),
        livekit_service.clone(),
    );
    let list_servers_by_user_usecase = ListServersByUser::new(server_repository.clone());
    let list_server_members_usecase = ListServerMembers::new(server_repository.clone());
    let list_server_channels_usecase = ListServerChannels::new(server_repository.clone());
    let get_voice_presence_usecase =
        GetVoicePresence::new(server_repository.clone(), channel_repository.clone());
    let update_stream_viewer_usecase =
        UpdateStreamViewer::new(server_repository.clone(), channel_repository.clone());

    let friendship_repository =
        Arc::new(PostgresFriendshipRepository::new(postgres_pool.clone()));
    let start_call_usecase = StartCall::new(
        call_repository.clone(),
        livekit_service.clone(),
        friendship_repository.clone(),
        user_repository.clone(),
    );
    let accept_call_usecase = AcceptCall::new(
        call_repository.clone(),
        livekit_service.clone(),
        user_repository.clone(),
    );
    let decline_call_usecase = DeclineCall::new(call_repository.clone());
    let leave_call_usecase = LeaveCall::new(call_repository.clone());
    let add_call_participants_usecase =
        AddCallParticipants::new(call_repository.clone(), friendship_repository.clone());
    let get_active_call_usecase = GetActiveCall::new(
        call_repository.clone(),
        livekit_service.clone(),
        user_repository.clone(),
    );
    let get_active_call_with_usecase = GetActiveCallWith::new(call_repository.clone());
    let join_call_usecase = JoinCall::new(
        call_repository.clone(),
        livekit_service.clone(),
        user_repository.clone(),
    );
    let send_friend_request_usecase = SendFriendRequest::new(
        friendship_repository.clone(),
        user_repository.clone(),
    );
    let accept_friend_request_usecase =
        AcceptFriendRequest::new(friendship_repository.clone());
    let remove_friendship_usecase = RemoveFriendship::new(friendship_repository.clone());
    let list_friends_usecase = ListFriends::new(friendship_repository.clone());

    let search_users_usecase = SearchUsers::new(user_repository.clone());
    let get_me_usecase = GetMe::new(user_repository.clone());
    let update_user_locale_usecase = UpdateUserLocale::new(user_repository.clone());
    let read_state_repository = Arc::new(PostgresReadStateRepository::new(postgres_pool.clone()));
    let mark_channel_read_usecase =
        MarkChannelRead::new(read_state_repository.clone(), channel_repository.clone());
    let mark_dm_read_usecase =
        MarkDmRead::new(read_state_repository.clone(), friendship_repository.clone());
    let get_unread_counts_usecase = GetUnreadCounts::new(read_state_repository.clone());
    let get_dm_read_receipt_usecase = GetDmReadReceipt::new(
        read_state_repository.clone(),
        friendship_repository.clone(),
    );
    let get_channel_read_receipts_usecase = GetChannelReadReceipts::new(
        read_state_repository.clone(),
        channel_repository.clone(),
    );

    let nats_core = nats_jetstream.client().clone();
    let presence_preference_subject = format!(
        "{}.realtime.presence.preference",
        nats_config.subject_prefix.trim_end_matches('.'),
    );

    let dm_repository = Arc::new(PostgresDmMessageRepository::new(postgres_pool.clone()));
    let log_call_usecase =
        LogCall::new(dm_repository.clone(), friendship_repository.clone());
    let send_dm_message_usecase = SendDmMessage::new(
        dm_repository.clone(),
        friendship_repository.clone(),
        attachment_repository.clone(),
    );
    let edit_dm_message_usecase = EditDmMessage::new(
        dm_repository.clone(),
        attachment_repository.clone(),
        minio_storage.clone(),
    );
    let delete_dm_message_usecase = DeleteDmMessage::new(
        dm_repository.clone(),
        attachment_repository.clone(),
        minio_storage.clone(),
    );
    let list_dm_previews_usecase = ListDmPreviews::new(dm_repository.clone());
    let list_dm_messages_usecase = ListDmMessages::new(
        dm_repository.clone(),
        friendship_repository.clone(),
    );
    let search_dm_messages_usecase = SearchDmMessages::new(
        dm_repository.clone(),
        friendship_repository.clone(),
    );
    let send_server_invite_usecase = SendServerInvite::new(
        dm_repository.clone(),
        friendship_repository.clone(),
        server_repository.clone(),
    );
    let accept_server_invite_usecase =
        AcceptServerInvite::new(dm_repository.clone(), server_repository.clone());
    let decline_server_invite_usecase = DeclineServerInvite::new(dm_repository.clone());
    let list_sent_server_invites_usecase =
        ListSentServerInvites::new(dm_repository.clone(), server_repository.clone());

    let app_state = web::Data::new(AppState::new(
        create_channel_usecase,
        join_voice_channel_usecase,
        start_call_usecase,
        accept_call_usecase,
        decline_call_usecase,
        leave_call_usecase,
        add_call_participants_usecase,
        get_active_call_usecase,
        get_active_call_with_usecase,
        join_call_usecase,
        log_call_usecase,
        mute_channel_usecase,
        unmute_channel_usecase,
        delete_channel_usecase,
        list_channel_participants_usecase,
        join_text_channel_usecase,
        send_channel_message_usecase,
        edit_channel_message_usecase,
        delete_channel_message_usecase,
        list_channel_messages_usecase,
        search_channel_messages_usecase,
        register_user_usecase,
        issue_verification_code_usecase,
        login_user_usecase,
        google_login_usecase,
        confirm_registration_usecase,
        refresh_tokens_usecase,
        logout_usecase,
        handle_livekit_webhook_usecase,
        create_server_usecase,
        delete_server_usecase,
        update_server_usecase,
        create_invite_code_usecase,
        list_invite_codes_usecase,
        revoke_invite_code_usecase,
        redeem_invite_code_usecase,
        leave_server_usecase,
        list_servers_by_user_usecase,
        list_server_members_usecase,
        list_server_channels_usecase,
        get_voice_presence_usecase,
        update_stream_viewer_usecase,
        send_friend_request_usecase,
        accept_friend_request_usecase,
        remove_friendship_usecase,
        list_friends_usecase,
        search_users_usecase,
        get_me_usecase,
        update_user_locale_usecase,
        mark_channel_read_usecase,
        mark_dm_read_usecase,
        get_unread_counts_usecase,
        get_dm_read_receipt_usecase,
        get_channel_read_receipts_usecase,
        nats_core,
        presence_preference_subject,
        send_dm_message_usecase,
        upload_attachment_usecase,
        upload_semaphore,
        get_attachment_usecase,
        delete_attachment_usecase,
        list_message_attachments_usecase,
        update_user_avatar_usecase,
        delete_user_avatar_usecase,
        get_user_avatar_usecase,
        update_server_logo_usecase,
        get_server_logo_usecase,
        edit_dm_message_usecase,
        delete_dm_message_usecase,
        list_dm_previews_usecase,
        list_dm_messages_usecase,
        search_dm_messages_usecase,
        send_server_invite_usecase,
        accept_server_invite_usecase,
        decline_server_invite_usecase,
        list_sent_server_invites_usecase,
        server_config.jwt_secret,
        server_config.trust_forwarded_headers,
    ));

    let rate_limiter = create_rate_limiter(rate_limiting_config)
        .ok_or("Failed to create rate limiter: invalid parameters")?;
    let rate_limiter_data = web::Data::new(rate_limiter);

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
