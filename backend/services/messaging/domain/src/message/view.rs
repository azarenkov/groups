use chrono::{DateTime, Utc};

use crate::{channel::id::ChannelId, message::id::MessageId, message::system::SystemMessage};
use domain::user::id::UserId;

#[derive(Debug, Clone)]
pub struct ReplyPreviewView {
    pub message_id: uuid::Uuid,
    pub author_id: UserId,
    pub author_name: String,
    pub content: String,
    pub has_attachments: bool,
    pub voice_duration_ms: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct MessageView {
    pub id: MessageId,
    pub channel_id: ChannelId,
    pub author_id: UserId,
    pub author_name: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub system: Option<SystemMessage>,
    pub reply_to: Option<uuid::Uuid>,
    pub forwarded_from: Option<crate::message::forward::ForwardInfo>,
}
