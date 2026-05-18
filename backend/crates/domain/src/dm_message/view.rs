use chrono::{DateTime, Utc};

use crate::{dm_message::id::DmMessageId, user::id::UserId};

#[derive(Debug, Clone)]
pub struct DmMessageView {
    pub id: DmMessageId,
    pub sender_id: UserId,
    pub sender_name: String,
    pub recipient_id: UserId,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
}
