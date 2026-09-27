use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimRequestV1 {
    pub attachment_ids: Vec<Uuid>,
    pub owner_id: Uuid,
    pub claim_token: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseRequestV1 {
    pub attachment_ids: Vec<Uuid>,
    pub claim_token: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimedAttachmentV1 {
    pub id: Uuid,
    pub file_name: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub storage_key: String,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub voice_duration_ms: Option<i32>,
    #[serde(default)]
    pub voice_waveform: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkedRequestV1 {
    pub attachment_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkedResponseV1 {
    pub linked_ids: Vec<Uuid>,
}
