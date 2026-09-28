use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceMeta {
    pub duration_ms: i32,
    pub waveform: Vec<u8>,
}

impl VoiceMeta {
    pub fn from_columns(duration_ms: Option<i32>, waveform: Option<Vec<u8>>) -> Option<Self> {
        match (duration_ms, waveform) {
            (Some(duration_ms), Some(waveform)) => Some(Self { duration_ms, waveform }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Attachment {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub file_name: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub storage_key: String,
    pub created_at: DateTime<Utc>,
    pub voice: Option<VoiceMeta>,
}

#[derive(Debug, Clone)]
pub struct AttachmentLink {
    pub id: Uuid,
    pub file_name: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub storage_key: String,
    pub created_at: DateTime<Utc>,
    pub voice: Option<VoiceMeta>,
}

impl From<&Attachment> for AttachmentLink {
    fn from(a: &Attachment) -> Self {
        Self {
            id: a.id,
            file_name: a.file_name.clone(),
            content_type: a.content_type.clone(),
            size_bytes: a.size_bytes,
            storage_key: a.storage_key.clone(),
            created_at: a.created_at,
            voice: a.voice.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentSnapshot {
    pub id: Uuid,
    pub file_name: String,
    pub content_type: String,
    pub size_bytes: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<VoiceMeta>,
}

impl From<&Attachment> for AttachmentSnapshot {
    fn from(a: &Attachment) -> Self {
        Self {
            id: a.id,
            file_name: a.file_name.clone(),
            content_type: a.content_type.clone(),
            size_bytes: a.size_bytes,
            voice: a.voice.clone(),
        }
    }
}

impl From<&AttachmentLink> for AttachmentSnapshot {
    fn from(a: &AttachmentLink) -> Self {
        Self {
            id: a.id,
            file_name: a.file_name.clone(),
            content_type: a.content_type.clone(),
            size_bytes: a.size_bytes,
            voice: a.voice.clone(),
        }
    }
}
