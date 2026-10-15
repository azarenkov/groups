use chrono::{DateTime, Utc};
use messaging_domain::dm_message::view::DmMessageView;
use uuid::Uuid;

#[derive(Debug)]
pub struct DmMessageDto {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub sender_name: String,
    pub recipient_id: Uuid,
    pub content: String,
    pub call_status: Option<String>,
    pub call_duration_seconds: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub reply_to_message_id: Option<Uuid>,
    pub forwarded_from: Option<messaging_domain::message::forward::ForwardInfo>,
    pub sticker: Option<domain::sticker::StickerSnapshot>,
}

impl From<DmMessageView> for DmMessageDto {
    fn from(value: DmMessageView) -> Self {
        Self {
            id: value.id.uuid(),
            sender_id: value.sender_id.uuid(),
            sender_name: value.sender_name,
            recipient_id: value.recipient_id.uuid(),
            content: value.content,
            call_status: value.call_status,
            call_duration_seconds: value.call_duration_seconds,
            created_at: value.created_at,
            edited_at: value.edited_at,
            reply_to_message_id: value.reply_to,
            forwarded_from: value.forwarded_from,
            sticker: value.sticker,
        }
    }
}
