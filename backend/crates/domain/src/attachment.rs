pub mod repository;

use chrono::{DateTime, Utc};
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
