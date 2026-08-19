use chrono::{DateTime, Utc};
use domain::dm_message::DmMessage;
use uuid::Uuid;

#[derive(Debug)]
pub struct SendDmMessageResponse {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub recipient_id: Uuid,
    pub content: String,
    pub invite_server_id: Option<Uuid>,
    pub invite_server_name: Option<String>,
    pub attachments: Vec<domain::attachment::AttachmentLink>,
    pub created_at: DateTime<Utc>,
}

impl SendDmMessageResponse {
    pub fn from_invite(value: &DmMessage, invite_server_name: String) -> Self {
        Self {
            id: value.id().uuid(),
            sender_id: value.sender_id().uuid(),
            recipient_id: value.recipient_id().uuid(),
            content: value.content().as_str().to_string(),
            attachments: Vec::new(),
            invite_server_id: value.invite_server_id().map(|id| id.uuid()),
            invite_server_name: Some(invite_server_name),
            created_at: value.created_at(),
        }
    }
}

impl From<&DmMessage> for SendDmMessageResponse {
    fn from(value: &DmMessage) -> Self {
        Self {
            id: value.id().uuid(),
            sender_id: value.sender_id().uuid(),
            recipient_id: value.recipient_id().uuid(),
            content: value.content().as_str().to_string(),
            attachments: Vec::new(),
            invite_server_id: value.invite_server_id().map(|id| id.uuid()),
            invite_server_name: None,
            created_at: value.created_at(),
        }
    }
}
