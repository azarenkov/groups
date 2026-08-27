use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{channel::id::ChannelId, message::content::MessageContent, message::id::MessageId};
use domain::{event::EventId, user::id::UserId};

#[derive(Debug, Serialize)]
pub struct MessageSent {
    pub id: EventId,
    pub message_id: MessageId,
    pub channel_id: ChannelId,
    pub author_id: UserId,
    pub content: MessageContent,
    pub occurred_at: DateTime<Utc>,
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
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MessageEdited {
    pub id: EventId,
    pub message_id: MessageId,
    pub channel_id: ChannelId,
    pub new_content: MessageContent,
    pub occurred_at: DateTime<Utc>,
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
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MessageDeleted {
    pub id: EventId,
    pub message_id: MessageId,
    pub channel_id: ChannelId,
    pub deleted_by: UserId,
    pub occurred_at: DateTime<Utc>,
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
        }
    }
}
