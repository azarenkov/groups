use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    server::{
        events::lifecycle::{ServerCreated, ServerDeleted},
        id::ServerId,
    },
    shared::outbox::{
        outbox_record::{AggregateType, OutboxRecord},
        to_outbox::{OutboxError, ToOutboxRecord},
    },
};

pub mod lifecycle;

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ServerEvent {
    Created(ServerCreated),
    Deleted(ServerDeleted),
}

impl ServerEvent {
    pub fn server_id(&self) -> &ServerId {
        match self {
            Self::Created(e) => &e.server_id,
            Self::Deleted(e) => &e.server_id,
        }
    }

    pub fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            Self::Created(e) => e.occurred_at,
            Self::Deleted(e) => e.occurred_at,
        }
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Created(_) => "created",
            Self::Deleted(_) => "deleted",
        }
    }
}

impl ToOutboxRecord for ServerEvent {
    fn to_outbox_record(&self) -> Result<OutboxRecord, OutboxError> {
        Ok(OutboxRecord {
            id: Uuid::new_v4(),
            aggregate_id: self.server_id().uuid(),
            aggregate_type: AggregateType::Server,
            parent_id: None,
            parent_type: None,
            event_type: self.event_type().to_string(),
            payload: serde_json::to_value(self)?,
            occurred_at: self.occurred_at(),
            request_id: None,
        })
    }
}
