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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoNoteMeta {
    pub duration_ms: i32,
    pub size: i32,
    #[serde(serialize_with = "poster_base64")]
    pub poster: Vec<u8>,
}

fn poster_base64<S: serde::Serializer>(poster: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    use base64::Engine;
    serializer.serialize_str(&base64::engine::general_purpose::STANDARD.encode(poster))
}

impl VideoNoteMeta {
    pub fn from_columns(
        duration_ms: Option<i32>,
        size: Option<i32>,
        poster: Option<Vec<u8>>,
    ) -> Option<Self> {
        match (duration_ms, size, poster) {
            (Some(duration_ms), Some(size), Some(poster)) => Some(Self { duration_ms, size, poster }),
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
    pub video_note: Option<VideoNoteMeta>,
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
    pub video_note: Option<VideoNoteMeta>,
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
            video_note: a.video_note.clone(),
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_note: Option<VideoNoteMeta>,
}

impl From<&Attachment> for AttachmentSnapshot {
    fn from(a: &Attachment) -> Self {
        Self {
            id: a.id,
            file_name: a.file_name.clone(),
            content_type: a.content_type.clone(),
            size_bytes: a.size_bytes,
            voice: a.voice.clone(),
            video_note: a.video_note.clone(),
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
            video_note: a.video_note.clone(),
        }
    }
}
