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
    pub invite_server_id: Option<Uuid>,
    pub invite_server_name: Option<String>,
    pub call_status: Option<String>,
    pub call_duration_seconds: Option<i32>,
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
            invite_server_id: value.invite_server_id.map(|id| id.uuid()),
            invite_server_name: value.invite_server_name,
            call_status: value.call_status,
            call_duration_seconds: value.call_duration_seconds,
            created_at: value.created_at,
            edited_at: value.edited_at,
        }
    }
}
