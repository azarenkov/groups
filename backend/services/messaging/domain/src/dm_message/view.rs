use chrono::{DateTime, Utc};

use crate::{dm_message::id::DmMessageId};
use domain::user::id::UserId;

#[derive(Debug, Clone)]
pub struct DmMessageView {
    pub id: DmMessageId,
    pub sender_id: UserId,
    pub sender_name: String,
    pub recipient_id: UserId,
    pub content: String,
    pub call_status: Option<String>,
    pub call_duration_seconds: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub reply_to: Option<uuid::Uuid>,
    pub forwarded_from: Option<crate::message::forward::ForwardInfo>,
    pub sticker: Option<domain::sticker::StickerSnapshot>,
}
