use chrono::{DateTime, Utc};

use domain::{attachment::AttachmentSnapshot, user::id::UserId};

use crate::{channel::id::ChannelId, message::events::lifecycle::MessageDeleted, message::events::lifecycle::MessageEdited, message::events::lifecycle::MessageSent, message::id::MessageId};

pub mod lifecycle;

#[derive(Debug)]
pub enum MessageDomainEvent {
    Sent(MessageSent),
    Edited(MessageEdited),
    Deleted(MessageDeleted),
}

impl MessageDomainEvent {
    pub fn message_id(&self) -> &MessageId {
        match self {
            Self::Sent(e) => &e.message_id,
            Self::Edited(e) => &e.message_id,
            Self::Deleted(e) => &e.message_id,
        }
    }

    pub fn channel_id(&self) -> &ChannelId {
        match self {
            Self::Sent(e) => &e.channel_id,
            Self::Edited(e) => &e.channel_id,
            Self::Deleted(e) => &e.channel_id,
        }
    }

    pub fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            Self::Sent(e) => e.occurred_at,
            Self::Edited(e) => e.occurred_at,
            Self::Deleted(e) => e.occurred_at,
        }
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Sent(_) => "sent",
            Self::Edited(_) => "edited",
            Self::Deleted(_) => "deleted",
        }
    }

    pub fn with_recipients(mut self, recipient_ids: &[UserId]) -> Self {
        match &mut self {
            Self::Sent(e) => e.recipient_ids = recipient_ids.to_vec(),
            Self::Edited(e) => e.recipient_ids = recipient_ids.to_vec(),
            Self::Deleted(e) => e.recipient_ids = recipient_ids.to_vec(),
        }
        self
    }

    pub fn with_skipped(mut self, skipped_ids: &[UserId]) -> Self {
        if let Self::Edited(e) = &mut self {
            e.skipped_ids = skipped_ids.to_vec();
        }
        self
    }

    pub fn with_attachments(mut self, attachments: &[AttachmentSnapshot]) -> Self {
        match &mut self {
            Self::Sent(e) => e.attachments = attachments.to_vec(),
            Self::Edited(e) => e.attachments = attachments.to_vec(),
            Self::Deleted(_) => {}
        }
        self
    }

    pub fn with_mentions(mut self, mentions: &[UserId]) -> Self {
        if let Self::Sent(e) = &mut self {
            e.mentions = mentions.to_vec();
        }
        self
    }
}
