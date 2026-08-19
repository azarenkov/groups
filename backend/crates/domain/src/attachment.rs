pub mod repository;

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Attachment {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub file_name: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub storage_key: String,
    pub message_id: Option<Uuid>,
    pub dm_message_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentSnapshot {
    pub id: Uuid,
    pub file_name: String,
    pub content_type: String,
    pub size_bytes: i64,
}

impl From<&Attachment> for AttachmentSnapshot {
    fn from(a: &Attachment) -> Self {
        Self {
            id: a.id,
            file_name: a.file_name.clone(),
            content_type: a.content_type.clone(),
            size_bytes: a.size_bytes,
        }
    }
}
