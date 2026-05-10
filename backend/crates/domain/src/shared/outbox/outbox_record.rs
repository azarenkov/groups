use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug)]
pub struct OutboxRecord {
    pub id: Uuid,
    pub aggregate_id: Uuid,
    pub aggregate_type: AggregateType,
    pub parent_id: Option<Uuid>,
    pub parent_type: Option<AggregateType>,
    pub event_type: String,
    pub payload: Value,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AggregateType {
    Channel,
    Server,
    User,
}

impl AggregateType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Channel => "channel",
            Self::Server => "server",
            Self::User => "user",
        }
    }
}
