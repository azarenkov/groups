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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfileChangedV1 {
    pub user_id: Uuid,
    pub bio: String,
    pub show_activity: bool,
    #[serde(default = "default_true")]
    pub active: bool,
    #[serde(default)]
    pub timezone: String,
    pub occurred_at: DateTime<Utc>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserActivityChangedV1 {
    pub user_id: Uuid,
    #[serde(default)]
    pub game_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub started_at: Option<DateTime<Utc>>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationIssuedV1 {
    pub user_id: Uuid,
    pub verification_code_id: Uuid,
    pub channel: String,
    pub purpose: String,
    pub email: String,
    pub code: String,
    pub occurred_at: DateTime<Utc>,
    #[serde(default)]
    pub locale: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferenceUpdatedV1 {
    pub user_id: Uuid,
    pub scope_type: String,
    pub scope_id: Uuid,
    pub level: String,
    #[serde(default)]
    pub version: Option<i64>,
    #[serde(default)]
    pub recipients: Vec<Uuid>,
}
