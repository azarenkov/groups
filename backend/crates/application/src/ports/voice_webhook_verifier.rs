use chrono::{DateTime, Utc};
use domain::channel::events::participant::{ConnectionAbortReason, LeaveReason};
use uuid::Uuid;

#[derive(Debug)]
pub struct VerifiedVoiceWebhookEvent {
    pub external_id: String,
    pub occurred_at: DateTime<Utc>,
    pub event: VoiceWebhookEvent,
}

#[derive(Debug)]
pub enum VoiceWebhookEvent {
    ParticipantLeft {
        channel_id: Uuid,
        user_id: Uuid,
        reason: LeaveReason,
    },
    ParticipantConnectionAborted {
        channel_id: Uuid,
        user_id: Uuid,
        reason: ConnectionAbortReason,
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
