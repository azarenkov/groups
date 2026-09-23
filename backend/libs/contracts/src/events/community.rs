use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityCreatedV1 {
    pub community_id: Uuid,
    pub owner_id: Uuid,
    pub name: Value,
    #[serde(default)]
    pub description: Option<Value>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityUpdatedV1 {
    pub community_id: Uuid,
    pub name: Value,
    #[serde(default)]
    pub description: Option<Value>,
    pub member_ids: Vec<Uuid>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityOwnerChangedV1 {
    pub community_id: Uuid,
    pub owner_id: Uuid,
    pub previous_owner_id: Uuid,
    pub member_ids: Vec<Uuid>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityDeletedV1 {
    pub community_id: Uuid,
    pub owner_id: Uuid,
    #[serde(default)]
    pub member_ids: Option<Vec<Uuid>>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityMemberV1 {
    pub community_id: Uuid,
    pub user_id: Uuid,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityRolesChangedV1 {
    pub community_id: Uuid,
    #[serde(default)]
    pub recipients: Vec<Uuid>,
}
