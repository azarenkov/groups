use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaSourceV1 {
    Camera,
    Screen,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEventV1 {
    pub channel_id: Uuid,
    pub user_id: Uuid,
    pub session_id: String,
    pub occurred_at: DateTime<Utc>,
    pub inbox_source: String,
    pub inbox_external_id: String,
    #[serde(default)]
    pub reason: Option<Value>,
    #[serde(default)]
    pub media_source: Option<MediaSourceV1>,
    #[serde(default)]
    pub published: Option<bool>,
}
