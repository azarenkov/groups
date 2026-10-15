use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{dm_message::id::DmMessageId, message::content::MessageContent};
use domain::{attachment::AttachmentSnapshot, user::id::UserId};

#[derive(Debug)]
pub struct DmMessageSent {
    pub id: Uuid,
    pub message_id: DmMessageId,
    pub sender_id: UserId,
    pub recipient_id: UserId,
    pub content: MessageContent,
    pub occurred_at: DateTime<Utc>,
    pub call_status: Option<String>,
    pub call_duration_seconds: Option<i32>,
    pub read_by_recipient: bool,
    pub hidden_for: Vec<UserId>,
    pub attachments: Vec<AttachmentSnapshot>,
    pub reply: Option<crate::message::reply::ReplyPreview>,
    pub forward: Option<crate::message::forward::ForwardInfo>,
    pub sticker: Option<domain::sticker::StickerSnapshot>,
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
            call_status: None,
            call_duration_seconds: None,
            read_by_recipient: false,
            hidden_for: Vec::new(),
            attachments: Vec::new(),
            reply: None,
            forward: None,
            sticker: None,
        }
    }

    pub fn for_call(mut self, status: &str, duration_seconds: Option<i32>, read_by_recipient: bool) -> Self {
        self.call_status = Some(status.to_string());
        self.call_duration_seconds = duration_seconds;
        self.read_by_recipient = read_by_recipient;
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
    pub hidden_for: Vec<UserId>,
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
            hidden_for: Vec::new(),
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
