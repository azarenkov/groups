use chrono::{DateTime, Utc};
use domain::channel::events::participant::LeaveReason;
use uuid::Uuid;

#[derive(Debug)]
pub struct LeaveVoiceConnectionRequest {
    pub channel_id: Uuid,
    pub user_id: Uuid,
    pub reason: LeaveReason,
    pub occurred_at: DateTime<Utc>,
}
