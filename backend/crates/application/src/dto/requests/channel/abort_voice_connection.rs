use chrono::{DateTime, Utc};
use domain::channel::events::participant::ConnectionAbortReason;
use uuid::Uuid;

use crate::ports::voice_webhook_verifier::WebhookSource;

#[derive(Debug)]
pub struct AbortVoiceConnectionRequest {
    pub channel_id: Uuid,
    pub user_id: Uuid,
    pub session_id: String,
    pub reason: ConnectionAbortReason,
    pub occurred_at: DateTime<Utc>,
    pub inbox_source: WebhookSource,
    pub inbox_external_id: String,
}
