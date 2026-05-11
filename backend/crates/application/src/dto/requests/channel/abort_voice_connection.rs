use chrono::{DateTime, Utc};
use domain::channel::events::participant::ConnectionAbortReason;
use uuid::Uuid;

#[derive(Debug)]
pub struct AbortVoiceConnectionRequest {
    pub channel_id: Uuid,
    pub user_id: Uuid,
    pub reason: ConnectionAbortReason,
    pub occurred_at: DateTime<Utc>,
}
