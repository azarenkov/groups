use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresencePreferenceV1 {
    pub user_id: Uuid,
    pub appear_offline: bool,
    #[serde(default)]
    pub occurred_at: Option<chrono::DateTime<chrono::Utc>>,
}
