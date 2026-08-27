pub mod lifecycle;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{friendship::events::lifecycle::FriendRequestAccepted, friendship::events::lifecycle::FriendRequestSent, friendship::events::lifecycle::FriendshipRemoved};
use domain::{shared::outbox::outbox_record::AggregateType, shared::outbox::outbox_record::OutboxRecord, shared::outbox::to_outbox::OutboxError, shared::outbox::to_outbox::ToOutboxRecord, user::id::UserId};

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum FriendshipDomainEvent {
    RequestSent(FriendRequestSent),
    RequestAccepted(FriendRequestAccepted),
    Removed(FriendshipRemoved),
}

impl FriendshipDomainEvent {
    fn event_type(&self) -> &'static str {
        match self {
            Self::RequestSent(_) => "request_sent",
            Self::RequestAccepted(_) => "request_accepted",
            Self::Removed(_) => "removed",
        }
    }

    fn aggregate_user_id(&self) -> UserId {
        match self {
            Self::RequestSent(e) => e.sender_id,
            Self::RequestAccepted(e) => e.accepter_id,
            Self::Removed(e) => e.user_id,
        }
    }

    fn parent_user_id(&self) -> UserId {
        match self {
            Self::RequestSent(e) => e.receiver_id,
            Self::RequestAccepted(e) => e.sender_id,
            Self::Removed(e) => e.other_id,
        }
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            Self::RequestSent(e) => e.occurred_at,
            Self::RequestAccepted(e) => e.occurred_at,
            Self::Removed(e) => e.occurred_at,
        }
    }
}

impl ToOutboxRecord for FriendshipDomainEvent {
    fn to_outbox_record(&self) -> Result<OutboxRecord, OutboxError> {
        Ok(OutboxRecord {
            id: uuid::Uuid::new_v4(),
            aggregate_id: self.aggregate_user_id().uuid(),
            aggregate_type: AggregateType::User,
            parent_id: Some(self.parent_user_id().uuid()),
            parent_type: Some(AggregateType::User),
            event_type: self.event_type().to_string(),
            payload: serde_json::to_value(self)?,
            occurred_at: self.occurred_at(),
            request_id: None,
        })
    }
}
