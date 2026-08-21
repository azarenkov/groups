use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRenamedV1 {
    pub user_id: Uuid,
    pub name: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserNameV1 {
    pub user_id: Uuid,
    pub name: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLifecycleV1 {
    pub user_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    #[serde(default)]
    pub name: Option<String>,
}
