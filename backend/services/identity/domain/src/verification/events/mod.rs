use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use domain::{shared::outbox::outbox_record::AggregateType, shared::outbox::outbox_record::OutboxRecord, shared::outbox::to_outbox::OutboxError, shared::outbox::to_outbox::ToOutboxRecord};
use crate::{user::id::UserId, verification::events::issued::VerificationCodeIssued};

pub mod issued;

#[derive(Debug, Serialize, Deserialize)]
pub enum VerificationEvent {
    Issued(VerificationCodeIssued),
}

impl VerificationEvent {
    pub fn user_id(&self) -> &UserId {
        match self {
            Self::Issued(e) => &e.user_id,
        }
    }

    pub fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            Self::Issued(e) => e.occurred_at,
        }
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Issued(_) => "verification.issued",
        }
    }
}

impl ToOutboxRecord for VerificationEvent {
    fn to_outbox_record(&self) -> Result<OutboxRecord, OutboxError> {
        Ok(OutboxRecord {
            id: Uuid::new_v4(),
            aggregate_id: self.user_id().uuid(),
            aggregate_type: AggregateType::User,
            parent_id: None,
            parent_type: None,
            event_type: self.event_type().to_string(),
            payload: serde_json::to_value(self)?,
            occurred_at: self.occurred_at(),
            request_id: None,
        })
    }
}
