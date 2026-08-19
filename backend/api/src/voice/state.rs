use application::usecases::{
    call::{
        accept_call::AcceptCall, add_participants::AddCallParticipants,
        decline_call::DeclineCall, get_active_call::GetActiveCall,
        get_active_call_with::GetActiveCallWith, join_call::JoinCall, leave_call::LeaveCall,
        log_call::LogCall, start_call::StartCall,
    },
    webhook::voice::HandleVoiceWebhook,
};
use infrastructure::{
    database::postgres::repositories::{
        call::PostgresCallRepository, channel::PostgresChannelRepository,
        dm_message::PostgresDmMessageRepository, friendship::PostgresFriendshipRepository,
        user_profile::PostgresUserProfileRepository,
    },
    voice::{livekit_service::LiveKitService, livekit_webhook_receiver::LiveKitWebhookReceiver},
};

pub struct AppState {
    pub start_call_usecase: StartCall<
        PostgresCallRepository,
        LiveKitService,
        PostgresFriendshipRepository,
        PostgresUserProfileRepository,
    >,
    pub accept_call_usecase:
        AcceptCall<PostgresCallRepository, LiveKitService, PostgresUserProfileRepository>,
    pub decline_call_usecase: DeclineCall<PostgresCallRepository>,
    pub leave_call_usecase: LeaveCall<PostgresCallRepository>,
    pub add_call_participants_usecase:
        AddCallParticipants<PostgresCallRepository, PostgresFriendshipRepository>,
    pub get_active_call_usecase:
        GetActiveCall<PostgresCallRepository, LiveKitService, PostgresUserProfileRepository>,
    pub get_active_call_with_usecase: GetActiveCallWith<PostgresCallRepository>,
    pub join_call_usecase:
        JoinCall<PostgresCallRepository, LiveKitService, PostgresUserProfileRepository>,
    pub log_call_usecase: LogCall<PostgresDmMessageRepository, PostgresFriendshipRepository>,
    pub handle_livekit_webhook_usecase: HandleVoiceWebhook<
        LiveKitWebhookReceiver,
        PostgresChannelRepository,
        PostgresCallRepository,
    >,
}
