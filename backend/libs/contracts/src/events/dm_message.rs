use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmMessageSentV1 {
    pub message_id: Uuid,
    pub sender_id: Uuid,
    pub recipient_id: Uuid,
    pub content: Value,
    pub occurred_at: DateTime<Utc>,
    #[serde(default)]
    pub invite_community_id: Option<Uuid>,
    #[serde(default)]
    pub invite_community_name: Option<String>,
    #[serde(default)]
    pub call_status: Option<String>,
    #[serde(default)]
    pub call_duration_seconds: Option<i32>,
    #[serde(default)]
    pub attachments: Option<Value>,
    #[serde(default)]
    pub reply_to: Option<Value>,
    #[serde(default)]
    pub forwarded_from: Option<Value>,
    #[serde(default)]
    pub sticker: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmMessageEditedV1 {
    pub message_id: Uuid,
    pub sender_id: Uuid,
    pub new_content: Value,
    pub occurred_at: DateTime<Utc>,
    #[serde(default)]
    pub recipient_id: Option<Uuid>,
    #[serde(default)]
    pub attachments: Option<Value>,
    #[serde(default)]
    pub hidden_for: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmMessageDeletedV1 {
    pub message_id: Uuid,
    pub sender_id: Uuid,
    pub recipient_id: Uuid,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmReadV1 {
    pub reader_id: Uuid,
    #[serde(default)]
    pub peer_id: Option<Uuid>,
    pub read_at: DateTime<Utc>,
    pub recipients: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmConversationClearedV1 {
    pub sender_id: Uuid,
    pub recipient_id: Uuid,
    pub scope: String,
    pub cleared_at: DateTime<Utc>,
    pub recipients: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmConversationHiddenChangedV1 {
    pub user_id: Uuid,
    pub peer_id: Uuid,
    pub hidden: bool,
    pub occurred_at: DateTime<Utc>,
}
