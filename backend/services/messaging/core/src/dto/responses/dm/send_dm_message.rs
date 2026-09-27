use chrono::{DateTime, Utc};
use messaging_domain::dm_message::DmMessage;
use uuid::Uuid;

#[derive(Debug)]
pub struct SendDmMessageResponse {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub recipient_id: Uuid,
    pub content: String,
    pub invite_community_id: Option<Uuid>,
    pub invite_community_name: Option<String>,
    pub attachments: Vec<domain::attachment::AttachmentLink>,
    pub created_at: DateTime<Utc>,
    pub reply: Option<crate::dto::responses::reply::ReplyPreviewDto>,
    pub forwarded_from: Option<messaging_domain::message::forward::ForwardInfo>,
}

impl SendDmMessageResponse {
    pub fn from_invite(value: &DmMessage, invite_community_name: String) -> Self {
        Self {
            id: value.id().uuid(),
            sender_id: value.sender_id().uuid(),
            recipient_id: value.recipient_id().uuid(),
            content: value.content().as_str().to_string(),
            attachments: Vec::new(),
            invite_community_id: value.invite_community_id().map(|id| id.uuid()),
            invite_community_name: Some(invite_community_name),
            created_at: value.created_at(),
            reply: value
                .reply()
                .map(|r| crate::dto::responses::reply::ReplyPreviewDto::from_preview(r, None)),
            forwarded_from: value.forward().cloned(),
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
            invite_community_id: value.invite_community_id().map(|id| id.uuid()),
            invite_community_name: None,
            created_at: value.created_at(),
            reply: value
                .reply()
                .map(|r| crate::dto::responses::reply::ReplyPreviewDto::from_preview(r, None)),
            forwarded_from: value.forward().cloned(),
        }
    }
}
