use chrono::{DateTime, Utc};
use domain::dm_message::DmMessage;
use uuid::Uuid;

#[derive(Debug)]
pub struct SendDmMessageResponse {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub recipient_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

impl From<&DmMessage> for SendDmMessageResponse {
    fn from(value: &DmMessage) -> Self {
        Self {
            id: value.id().uuid(),
            sender_id: value.sender_id().uuid(),
            recipient_id: value.recipient_id().uuid(),
            content: value.content().as_str().to_string(),
            created_at: value.created_at(),
        }
    }
}
