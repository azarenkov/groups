use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct OutboxRecord {
    pub id: Uuid,
    pub aggregate_id: Uuid,
    pub aggregate_type: AggregateType,
    pub parent_id: Option<Uuid>,
    pub parent_type: Option<AggregateType>,
    pub event_type: String,
    pub payload: Value,
    pub occurred_at: DateTime<Utc>,
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionSequence {
    pub partition: String,
    pub seq: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregateType {
    Channel,
    Community,
    User,
    Message,
    DmMessage,
    Storage,
}

impl AggregateType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Channel => "channel",
            Self::Community => "community",
            Self::User => "user",
            Self::Message => "message",
            Self::DmMessage => "dm_message",
            Self::Storage => "storage",
        }
    }
}
