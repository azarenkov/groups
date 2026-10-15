use chrono::{DateTime, Utc};
use messaging_domain::dm_message::DmMessage;
use uuid::Uuid;

#[derive(Debug)]
pub struct SendDmMessageResponse {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub recipient_id: Uuid,
    pub content: String,
    pub attachments: Vec<domain::attachment::AttachmentLink>,
    pub created_at: DateTime<Utc>,
    pub reply: Option<crate::dto::responses::reply::ReplyPreviewDto>,
    pub forwarded_from: Option<messaging_domain::message::forward::ForwardInfo>,
    pub sticker: Option<domain::sticker::StickerSnapshot>,
}

impl From<&DmMessage> for SendDmMessageResponse {
    fn from(value: &DmMessage) -> Self {
        Self {
            id: value.id().uuid(),
            sender_id: value.sender_id().uuid(),
            recipient_id: value.recipient_id().uuid(),
            content: value.content().as_str().to_string(),
            attachments: Vec::new(),
            created_at: value.created_at(),
            reply: value
                .reply()
                .map(|r| crate::dto::responses::reply::ReplyPreviewDto::from_preview(r, None)),
            forwarded_from: value.forward().cloned(),
            sticker: value.sticker().cloned(),
        }
    }
}
