use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCreatedV1 {
    pub server_id: Uuid,
    pub owner_id: Uuid,
    pub name: Value,
    #[serde(default)]
    pub description: Option<Value>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerUpdatedV1 {
    pub server_id: Uuid,
    pub name: Value,
    #[serde(default)]
    pub description: Option<Value>,
    pub member_ids: Vec<Uuid>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerDeletedV1 {
    pub server_id: Uuid,
    pub owner_id: Uuid,
    #[serde(default)]
    pub member_ids: Option<Vec<Uuid>>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMemberV1 {
    pub server_id: Uuid,
    pub user_id: Uuid,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerRolesChangedV1 {
    pub server_id: Uuid,
    #[serde(default)]
    pub recipients: Vec<Uuid>,
}
