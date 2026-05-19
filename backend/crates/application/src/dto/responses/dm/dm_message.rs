use chrono::{DateTime, Utc};
use domain::dm_message::view::DmMessageView;
use uuid::Uuid;

#[derive(Debug)]
pub struct DmMessageDto {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub sender_name: String,
    pub recipient_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
}

impl From<DmMessageView> for DmMessageDto {
    fn from(value: DmMessageView) -> Self {
        Self {
            id: value.id.uuid(),
            sender_id: value.sender_id.uuid(),
            sender_name: value.sender_name,
            recipient_id: value.recipient_id.uuid(),
            content: value.content,
            created_at: value.created_at,
            edited_at: value.edited_at,
        }
    }
}
