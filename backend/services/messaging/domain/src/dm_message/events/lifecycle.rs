use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{dm_message::id::DmMessageId, message::content::MessageContent, server::id::ServerId};
use domain::{attachment::AttachmentSnapshot, user::id::UserId};

#[derive(Debug)]
pub struct DmMessageSent {
    pub id: Uuid,
    pub message_id: DmMessageId,
    pub sender_id: UserId,
    pub recipient_id: UserId,
    pub content: MessageContent,
    pub occurred_at: DateTime<Utc>,
    pub invite_server_id: Option<ServerId>,
    pub invite_server_name: Option<String>,
    pub call_status: Option<String>,
    pub call_duration_seconds: Option<i32>,
    pub attachments: Vec<AttachmentSnapshot>,
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
            invite_server_id: None,
            invite_server_name: None,
            call_status: None,
            call_duration_seconds: None,
            attachments: Vec::new(),
        }
    }

    pub fn for_call(mut self, status: &str, duration_seconds: Option<i32>) -> Self {
        self.call_status = Some(status.to_string());
        self.call_duration_seconds = duration_seconds;
        self
    }

    pub fn for_invite(mut self, server_id: ServerId, server_name: &str) -> Self {
        self.invite_server_id = Some(server_id);
        self.invite_server_name = Some(server_name.to_string());
        self
    }
}

#[derive(Debug)]
pub struct DmMessageEdited {
    pub id: Uuid,
    pub message_id: DmMessageId,
    pub sender_id: UserId,
    pub recipient_id: UserId,
    pub new_content: MessageContent,
    pub occurred_at: DateTime<Utc>,
    pub attachments: Vec<AttachmentSnapshot>,
}

impl DmMessageEdited {
    pub fn record(
        message_id: DmMessageId,
        sender_id: UserId,
        recipient_id: UserId,
        new_content: MessageContent,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            message_id,
            sender_id,
            recipient_id,
            new_content,
            occurred_at: now,
            attachments: Vec::new(),
        }
    }
}

#[derive(Debug)]
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
