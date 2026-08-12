use std::sync::Arc;

use chrono::Utc;
use domain::{
    attachment::{Attachment, repository::AttachmentRepository},
    shared::repository::error::RepositoryError,
    user::id::UserId,
};
use uuid::Uuid;

use crate::ports::file_storage::{FileStorage, FileStorageError};

pub const MAX_ATTACHMENT_BYTES: usize = 15 * 1024 * 1024;
pub const MAX_UNLINKED_COUNT: i64 = 50;
pub const MAX_UNLINKED_BYTES: i64 = 200 * 1024 * 1024;

const ALLOWED_PREFIXES: &[&str] = &["image/", "video/", "audio/", "text/"];
const ALLOWED_TYPES: &[&str] = &[
    "application/pdf",
    "application/zip",
    "application/x-zip-compressed",
    "application/gzip",
    "application/x-tar",
    "application/x-7z-compressed",
    "application/json",
    "application/msword",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "application/vnd.ms-excel",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "application/vnd.ms-powerpoint",
    "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    "application/octet-stream",
];

fn content_type_allowed(content_type: &str) -> bool {
    ALLOWED_PREFIXES.iter().any(|p| content_type.starts_with(p))
        || ALLOWED_TYPES.contains(&content_type)
}

#[derive(Debug, thiserror::Error)]
pub enum UploadAttachmentError {
    #[error("File is too large")]
    TooLarge,
    #[error("File is empty")]
    Empty,
    #[error("File type is not allowed")]
    TypeNotAllowed,
    #[error("Too many pending uploads")]
    QuotaExceeded,
    #[error("Storage error: {0}")]
    Storage(#[from] FileStorageError),
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct UploadAttachment<R: AttachmentRepository, S: FileStorage> {
    repository: Arc<R>,
    storage: Arc<S>,
}

impl<R: AttachmentRepository, S: FileStorage> UploadAttachment<R, S> {
    pub fn new(repository: Arc<R>, storage: Arc<S>) -> Self {
        Self {
            repository,
            storage,
        }
    }

    pub async fn execute(
        &self,
        owner_id: UserId,
        file_name: String,
        content_type: String,
        bytes: Vec<u8>,
    ) -> Result<Attachment, UploadAttachmentError> {
        if bytes.is_empty() {
            return Err(UploadAttachmentError::Empty);
        }
        if bytes.len() > MAX_ATTACHMENT_BYTES {
            return Err(UploadAttachmentError::TooLarge);
        }
        if !content_type_allowed(&content_type) {
            return Err(UploadAttachmentError::TypeNotAllowed);
        }

        let id = Uuid::new_v4();
        let storage_key = format!("attachments/{id}");
        let attachment = Attachment {
            id,
            owner_id: owner_id.uuid(),
            file_name,
            content_type,
            size_bytes: bytes.len() as i64,
            storage_key,
            message_id: None,
            dm_message_id: None,
            created_at: Utc::now(),
        };

        let reserved = self
            .repository
            .insert_reserving_quota(&attachment, MAX_UNLINKED_COUNT, MAX_UNLINKED_BYTES)
            .await?;
        if !reserved {
            return Err(UploadAttachmentError::QuotaExceeded);
        }

        if let Err(e) = self
            .storage
            .put(&attachment.storage_key, &attachment.content_type, bytes)
            .await
        {
            match self
                .repository
                .enqueue_cleanup(std::slice::from_ref(&attachment.storage_key))
                .await
            {
                Ok(()) => {
                    let _ = self.repository.delete_orphan_row(attachment.id).await;
                }
                Err(qe) => {
                    tracing::warn!(error = %qe, key = %attachment.storage_key, "Failed to enqueue cleanup after upload error; orphan sweep will reclaim");
                }
            }
            return Err(e.into());
        }
        Ok(attachment)
    }
}
