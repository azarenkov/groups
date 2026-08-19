use chrono::{DateTime, Utc};
use domain::attachment::{Attachment, AttachmentLink};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentResponse {
    pub id: Uuid,
    pub file_name: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub created_at: DateTime<Utc>,
}

impl From<Attachment> for AttachmentResponse {
    fn from(a: Attachment) -> Self {
        Self {
            id: a.id,
            file_name: a.file_name,
            content_type: a.content_type,
            size_bytes: a.size_bytes,
            created_at: a.created_at,
        }
    }
}

impl From<AttachmentLink> for AttachmentResponse {
    fn from(a: AttachmentLink) -> Self {
        Self {
            id: a.id,
            file_name: a.file_name,
            content_type: a.content_type,
            size_bytes: a.size_bytes,
            created_at: a.created_at,
        }
    }
}
