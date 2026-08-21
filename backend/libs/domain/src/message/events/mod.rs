use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    channel::id::ChannelId,
    message::{
        events::lifecycle::{MessageDeleted, MessageEdited, MessageSent},
        id::MessageId,
    },
    shared::outbox::{
        outbox_record::{AggregateType, OutboxRecord},
        to_outbox::{OutboxError, ToOutboxRecord},
    },
};

pub mod lifecycle;

#[derive(Debug, Serialize)]
#[serde(untagged)]
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
}

impl ToOutboxRecord for MessageDomainEvent {
    fn to_outbox_record(&self) -> Result<OutboxRecord, OutboxError> {
        Ok(OutboxRecord {
            id: Uuid::new_v4(),
            aggregate_id: self.message_id().uuid(),
            aggregate_type: AggregateType::Message,
            parent_id: Some(self.channel_id().uuid()),
            parent_type: Some(AggregateType::Channel),
            event_type: self.event_type().to_string(),
            payload: serde_json::to_value(self)?,
            occurred_at: self.occurred_at(),
            request_id: None,
        })
    }
}
