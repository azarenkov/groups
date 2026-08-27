pub mod lifecycle;

use chrono::{DateTime, Utc};

use crate::{dm_message::events::lifecycle::DmMessageDeleted, dm_message::events::lifecycle::DmMessageEdited, dm_message::events::lifecycle::DmMessageSent};
use domain::{attachment::AttachmentSnapshot, user::id::UserId};

#[derive(Debug)]
pub enum DmMessageDomainEvent {
    Sent(DmMessageSent),
    Edited(DmMessageEdited),
    Deleted(DmMessageDeleted),
}

impl DmMessageDomainEvent {
    pub fn with_attachments(mut self, attachments: &[AttachmentSnapshot]) -> Self {
        match &mut self {
            Self::Sent(e) => e.attachments = attachments.to_vec(),
            Self::Edited(e) => e.attachments = attachments.to_vec(),
            Self::Deleted(_) => {}
        }
        self
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Sent(_) => "sent",
            Self::Edited(_) => "edited",
            Self::Deleted(_) => "deleted",
        }
    }

    pub fn aggregate_id(&self) -> uuid::Uuid {
        match self {
            Self::Sent(e) => e.message_id.uuid(),
            Self::Edited(e) => e.message_id.uuid(),
            Self::Deleted(e) => e.message_id.uuid(),
        }
    }

    pub fn parent_id(&self) -> Option<UserId> {
        match self {
            Self::Sent(e) => Some(e.sender_id),
            Self::Edited(e) => Some(e.sender_id),
            Self::Deleted(e) => Some(e.sender_id),
        }
    }

    pub fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            Self::Sent(e) => e.occurred_at,
            Self::Edited(e) => e.occurred_at,
            Self::Deleted(e) => e.occurred_at,
        }
    }
}
