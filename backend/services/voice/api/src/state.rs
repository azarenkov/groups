use infrastructure_common::database::postgres::repositories::plan_directory::PostgresPlanDirectory;
use voice::usecases::{
    call::{
        accept_call::AcceptCall, add_participants::AddCallParticipants,
        decline_call::DeclineCall, get_active_call::GetActiveCall,
        get_active_call_with::GetActiveCallWith, join_call::JoinCall, leave_call::LeaveCall,
        start_call::StartCall,
    },
    webhook::voice::HandleVoiceWebhook,
};
use voice_infrastructure::{
    database::postgres::repositories::{
        call::PostgresCallRepository, user_profile::PostgresUserProfileRepository,
    },
    outbox::webhook_outbox::PostgresWebhookOutbox,
    media_client::friendship::MessagingFriendshipClient,
    voice::{livekit_service::LiveKitService, livekit_webhook_receiver::LiveKitWebhookReceiver},
};

pub struct AppState {
    pub start_call_usecase: StartCall<
        PostgresCallRepository,
        LiveKitService,
        MessagingFriendshipClient,
        PostgresUserProfileRepository,
    >,
    pub accept_call_usecase: AcceptCall<
        PostgresCallRepository,
        LiveKitService,
        MessagingFriendshipClient,
        PostgresUserProfileRepository,
    >,
    pub decline_call_usecase: DeclineCall<PostgresCallRepository>,
    pub leave_call_usecase: LeaveCall<PostgresCallRepository>,
    pub add_call_participants_usecase: AddCallParticipants<
        PostgresCallRepository,
        LiveKitService,
        MessagingFriendshipClient,
        PostgresUserProfileRepository,
    >,
    pub get_active_call_usecase: GetActiveCall<
        PostgresCallRepository,
        LiveKitService,
        MessagingFriendshipClient,
        PostgresUserProfileRepository,
    >,
    pub get_active_call_with_usecase:
        GetActiveCallWith<PostgresCallRepository, MessagingFriendshipClient>,
    pub join_call_usecase: JoinCall<
        PostgresCallRepository,
        LiveKitService,
        MessagingFriendshipClient,
        PostgresUserProfileRepository,
    >,
    pub handle_livekit_webhook_usecase: HandleVoiceWebhook<
        LiveKitWebhookReceiver,
        PostgresCallRepository,
        PostgresWebhookOutbox,
        PostgresUserProfileRepository,
        LiveKitService,
        MessagingFriendshipClient,
        PostgresPlanDirectory,
    >,
    pub call_repository: std::sync::Arc<PostgresCallRepository>,
}
