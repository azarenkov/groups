pub mod lifecycle;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{dm_message::events::lifecycle::DmMessageDeleted, dm_message::events::lifecycle::DmMessageEdited, dm_message::events::lifecycle::DmMessageSent};
use domain::{shared::outbox::outbox_record::AggregateType, shared::outbox::outbox_record::OutboxRecord, shared::outbox::to_outbox::OutboxError, shared::outbox::to_outbox::ToOutboxRecord, user::id::UserId};

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum DmMessageDomainEvent {
    Sent(DmMessageSent),
    Edited(DmMessageEdited),
    Deleted(DmMessageDeleted),
}

impl DmMessageDomainEvent {
    fn event_type(&self) -> &'static str {
        match self {
            Self::Sent(_) => "sent",
            Self::Edited(_) => "edited",
            Self::Deleted(_) => "deleted",
        }
    }

    fn aggregate_id(&self) -> uuid::Uuid {
        match self {
            Self::Sent(e) => e.message_id.uuid(),
            Self::Edited(e) => e.message_id.uuid(),
            Self::Deleted(e) => e.message_id.uuid(),
        }
    }

    fn parent_id(&self) -> Option<UserId> {
        match self {
            Self::Sent(e) => Some(e.sender_id),
            Self::Edited(e) => Some(e.sender_id),
            Self::Deleted(e) => Some(e.sender_id),
        }
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            Self::Sent(e) => e.occurred_at,
            Self::Edited(e) => e.occurred_at,
            Self::Deleted(e) => e.occurred_at,
        }
    }
}

impl ToOutboxRecord for DmMessageDomainEvent {
    fn to_outbox_record(&self) -> Result<OutboxRecord, OutboxError> {
        Ok(OutboxRecord {
            id: uuid::Uuid::new_v4(),
            aggregate_id: self.aggregate_id(),
            aggregate_type: AggregateType::DmMessage,
            parent_id: self.parent_id().map(|u| u.uuid()),
            parent_type: Some(AggregateType::User),
            event_type: self.event_type().to_string(),
            payload: serde_json::to_value(self)?,
            occurred_at: self.occurred_at(),
            request_id: None,
        })
    }
}
