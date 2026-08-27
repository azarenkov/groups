use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{dm_message::id::DmMessageId, message::content::MessageContent};
use domain::user::id::UserId;

#[derive(Debug, Serialize)]
pub struct DmMessageSent {
    pub id: Uuid,
    pub message_id: DmMessageId,
    pub sender_id: UserId,
    pub recipient_id: UserId,
    pub content: MessageContent,
    pub occurred_at: DateTime<Utc>,
}

impl DmMessageSent {
    pub fn record(
        message_id: DmMessageId,
        sender_id: UserId,
        recipient_id: UserId,
        content: MessageContent,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            message_id,
            sender_id,
            recipient_id,
            content,
            occurred_at: now,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DmMessageEdited {
    pub id: Uuid,
    pub message_id: DmMessageId,
    pub sender_id: UserId,
    pub new_content: MessageContent,
    pub occurred_at: DateTime<Utc>,
}

impl DmMessageEdited {
    pub fn record(
        message_id: DmMessageId,
        sender_id: UserId,
        new_content: MessageContent,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            message_id,
            sender_id,
            new_content,
            occurred_at: now,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DmMessageDeleted {
    pub id: Uuid,
    pub message_id: DmMessageId,
    pub sender_id: UserId,
    pub recipient_id: UserId,
    pub occurred_at: DateTime<Utc>,
}

impl DmMessageDeleted {
    pub fn record(
        message_id: DmMessageId,
        sender_id: UserId,
        recipient_id: UserId,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            message_id,
            sender_id,
            recipient_id,
            occurred_at: now,
        }
    }
}
