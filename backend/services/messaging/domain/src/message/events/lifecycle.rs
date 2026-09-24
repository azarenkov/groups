use chrono::{DateTime, Utc};

use crate::{channel::id::ChannelId, message::content::MessageContent, message::id::MessageId, message::system::SystemMessage};
use domain::{attachment::AttachmentSnapshot, event::EventId, user::id::UserId};

#[derive(Debug)]
pub struct MessageSent {
    pub id: EventId,
    pub message_id: MessageId,
    pub channel_id: ChannelId,
    pub author_id: UserId,
    pub content: MessageContent,
    pub occurred_at: DateTime<Utc>,
    pub recipient_ids: Vec<UserId>,
    pub attachments: Vec<AttachmentSnapshot>,
    pub mentions: Vec<UserId>,
    pub system: Option<SystemMessage>,
    pub reply: Option<crate::message::reply::ReplyPreview>,
    pub forward: Option<crate::message::forward::ForwardInfo>,
}

impl MessageSent {
    pub fn record(
        message_id: MessageId,
        channel_id: ChannelId,
        author_id: UserId,
        content: MessageContent,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EventId::generate(),
            message_id,
            channel_id,
            author_id,
            content,
            occurred_at: now,
            recipient_ids: Vec::new(),
            attachments: Vec::new(),
            mentions: Vec::new(),
            system: None,
            reply: None,
            forward: None,
        }
    }
}

#[derive(Debug)]
pub struct MessageEdited {
    pub id: EventId,
    pub message_id: MessageId,
    pub channel_id: ChannelId,
    pub new_content: MessageContent,
    pub occurred_at: DateTime<Utc>,
    pub recipient_ids: Vec<UserId>,
    pub attachments: Vec<AttachmentSnapshot>,
}

impl MessageEdited {
    pub fn record(
        message_id: MessageId,
        channel_id: ChannelId,
        new_content: MessageContent,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EventId::generate(),
            message_id,
            channel_id,
            new_content,
            occurred_at: now,
            recipient_ids: Vec::new(),
            attachments: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct MessageDeleted {
    pub id: EventId,
    pub message_id: MessageId,
    pub channel_id: ChannelId,
    pub deleted_by: UserId,
    pub occurred_at: DateTime<Utc>,
    pub recipient_ids: Vec<UserId>,
}

impl MessageDeleted {
    pub fn record(
        message_id: MessageId,
        channel_id: ChannelId,
        deleted_by: UserId,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EventId::generate(),
            message_id,
            channel_id,
            deleted_by,
            occurred_at: now,
            recipient_ids: Vec::new(),
        }
    }
}
