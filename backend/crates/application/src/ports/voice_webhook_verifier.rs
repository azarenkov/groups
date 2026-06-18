use chrono::{DateTime, Utc};
use domain::channel::events::participant::{ConnectionAbortReason, LeaveReason, MediaSource};
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub enum WebhookSource {
    LiveKit,
    LiveKitPooler,
}

impl WebhookSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LiveKit => "livekit",
            Self::LiveKitPooler => "livekit_pooler",
        }
    }
}

#[derive(Debug)]
pub struct VerifiedVoiceWebhookEvent {
    pub source: WebhookSource,
    pub external_id: String,
    pub occurred_at: DateTime<Utc>,
    pub event: VoiceWebhookEvent,
}

#[derive(Debug)]
pub enum VoiceWebhookEvent {
    ParticipantJoined {
        channel_id: Uuid,
        user_id: Uuid,
        session_id: String,
    },
    ParticipantLeft {
        channel_id: Uuid,
        user_id: Uuid,
        session_id: String,
        reason: LeaveReason,
    },
    ParticipantConnectionAborted {
        channel_id: Uuid,
        user_id: Uuid,
        session_id: String,
        reason: ConnectionAbortReason,
    },
    TrackPublished {
        channel_id: Uuid,
        user_id: Uuid,
        session_id: String,
        source: MediaSource,
    },
    TrackUnpublished {
        channel_id: Uuid,
        user_id: Uuid,
        session_id: String,
        source: MediaSource,
    },
    Other,
}

#[derive(Debug, thiserror::Error)]
pub enum VoiceWebhookVerifierError {
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Failed to parse webhook body: {0}")]
    InvalidBody(String),
    #[error("Missing required field: {0}")]
    MissingField(&'static str),
    #[error("Invalid identifier in webhook: {0}")]
    InvalidIdentifier(String),
}

pub trait VoiceWebhookVerifier: Send + Sync {
    fn verify(
        &self,
        body: &str,
        auth_token: &str,
    ) -> Result<VerifiedVoiceWebhookEvent, VoiceWebhookVerifierError>;
}
