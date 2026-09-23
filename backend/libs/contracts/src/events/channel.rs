use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelCreatedV1 {
    pub channel_id: Uuid,
    pub community_id: Uuid,
    pub owner_id: Uuid,
    pub name: Value,
    pub kind: Value,
    pub visibility: Value,
    pub occurred_at: DateTime<Utc>,
    #[serde(default)]
    pub recipient_ids: Vec<Uuid>,
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelRenamedV1 {
    pub channel_id: Uuid,
    pub community_id: Uuid,
    pub old_name: Value,
    pub new_name: Value,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelDeletedV1 {
    pub channel_id: Uuid,
    pub community_id: Uuid,
    pub deleted_by: Uuid,
    pub occurred_at: DateTime<Utc>,
    #[serde(default)]
    pub recipient_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantJoinedV1 {
    pub channel_id: Uuid,
    pub community_id: Uuid,
    pub user_id: Uuid,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantLeftV1 {
    pub channel_id: Uuid,
    pub community_id: Uuid,
    pub user_id: Uuid,
    pub reason: Value,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantMutedV1 {
    pub channel_id: Uuid,
    pub community_id: Uuid,
    pub user_id: Uuid,
    pub muted_by: Uuid,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantUnmutedV1 {
    pub channel_id: Uuid,
    pub community_id: Uuid,
    pub user_id: Uuid,
    pub unmuted_by: Uuid,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantDeafenedV1 {
    pub channel_id: Uuid,
    pub community_id: Uuid,
    pub user_id: Uuid,
    pub deafened_by: Uuid,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantUndeafenedV1 {
    pub channel_id: Uuid,
    pub community_id: Uuid,
    pub user_id: Uuid,
    pub undeafened_by: Uuid,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantMediaV1 {
    pub channel_id: Uuid,
    pub community_id: Uuid,
    pub user_id: Uuid,
    pub source: String,
    pub enabled: bool,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantConnectionAbortedV1 {
    pub channel_id: Uuid,
    pub community_id: Uuid,
    pub user_id: Uuid,
    pub reason: Value,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamViewersV1 {
    pub channel_id: Uuid,
    pub community_id: Uuid,
    pub streamer_id: Uuid,
    pub count: i64,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextChannelJoinedV1 {
    pub channel_id: Uuid,
    pub community_id: Uuid,
    pub user_id: Uuid,
    pub joined_at: DateTime<Utc>,
    #[serde(default)]
    pub recipient_ids: Vec<Uuid>,
}
