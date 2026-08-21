use chrono::{DateTime, Utc};

use crate::{dm_message::id::DmMessageId, server::id::ServerId, user::id::UserId};

#[derive(Debug, Clone)]
pub struct DmMessageView {
    pub id: DmMessageId,
    pub sender_id: UserId,
    pub sender_name: String,
    pub recipient_id: UserId,
    pub content: String,
    pub invite_server_id: Option<ServerId>,
    pub invite_server_name: Option<String>,
    pub call_status: Option<String>,
    pub call_duration_seconds: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
}
