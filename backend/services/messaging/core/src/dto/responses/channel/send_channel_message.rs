use chrono::{DateTime, Utc};
use messaging_domain::message::Message;
use uuid::Uuid;

#[derive(Debug)]
pub struct SendChannelMessageResponse {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
    pub attachments: Vec<domain::attachment::AttachmentLink>,
    pub created_at: DateTime<Utc>,
    pub reply: Option<crate::dto::responses::reply::ReplyPreviewDto>,
    pub forwarded_from: Option<messaging_domain::message::forward::ForwardInfo>,
}

impl From<&Message> for SendChannelMessageResponse {
    fn from(value: &Message) -> Self {
        Self {
            id: value.id().uuid(),
            channel_id: value.channel_id().uuid(),
            author_id: value.author_id().uuid(),
            content: value.content().as_str().to_string(),
            attachments: Vec::new(),
            created_at: value.created_at(),
            reply: value
                .reply()
                .map(|r| crate::dto::responses::reply::ReplyPreviewDto::from_preview(r, None)),
            forwarded_from: value.forward().cloned(),
        }
    }
}
