use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessageSentV1 {
    pub message_id: Uuid,
    pub channel_id: Uuid,
    pub author_id: Uuid,
    pub content: Value,
    pub occurred_at: DateTime<Utc>,
    #[serde(default)]
    pub recipient_ids: Vec<Uuid>,
    #[serde(default)]
    pub attachments: Option<Value>,
    #[serde(default)]
    pub mentions: Option<Vec<Uuid>>,
    #[serde(default)]
    pub system: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessageEditedV1 {
    pub message_id: Uuid,
    pub channel_id: Uuid,
    pub new_content: Value,
    pub occurred_at: DateTime<Utc>,
    #[serde(default)]
    pub recipient_ids: Vec<Uuid>,
    #[serde(default)]
    pub attachments: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessageDeletedV1 {
    pub message_id: Uuid,
    pub channel_id: Uuid,
    pub deleted_by: Uuid,
    pub occurred_at: DateTime<Utc>,
    #[serde(default)]
    pub recipient_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelReadV1 {
    pub channel_id: Uuid,
    pub reader_id: Uuid,
    pub read_at: DateTime<Utc>,
    pub recipients: Vec<Uuid>,
}
