use std::sync::Arc;

use chrono::Utc;
use domain::{attachment::Attachment, user::id::UserId};
use ports::repository::RepositoryError;
use media_domain::attachment::repository::AttachmentRepository;
use uuid::Uuid;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use futures::StreamExt;
use media_domain::banner::BannerRepository;

use crate::ports::file_storage::{ByteStream, FileStorage, FileStorageError};

const MEGABYTE: u64 = 1024 * 1024;
pub const MAX_UNLINKED_COUNT: i64 = 50;
const MIN_UNLINKED_BYTES: u64 = 200 * MEGABYTE;
const MULTIPART_OVERHEAD_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UploadLimits {
    pub max_file_bytes: u64,
    pub max_unlinked_bytes: u64,
}

impl UploadLimits {
    pub fn for_tier(tier: &str) -> Self {
        let (max_file_bytes, per_message) = match tier {
            "premium" => (500 * MEGABYTE, 20),
            "pro" => (100 * MEGABYTE, 10),
            _ => (15 * MEGABYTE, 5),
        };
        Self {
            max_file_bytes,
            max_unlinked_bytes: MIN_UNLINKED_BYTES.max(max_file_bytes * per_message),
        }
    }

    pub fn max_request_bytes(&self) -> u64 {
        self.max_file_bytes + MULTIPART_OVERHEAD_BYTES
    }
}

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
    plans: Arc<dyn BannerRepository>,
}

impl<R: AttachmentRepository, S: FileStorage> UploadAttachment<R, S> {
    pub fn new(repository: Arc<R>, storage: Arc<S>, plans: Arc<dyn BannerRepository>) -> Self {
        Self {
            repository,
            storage,
            plans,
        }
    }

    pub async fn limits_for(&self, owner_id: &UserId) -> Result<UploadLimits, UploadAttachmentError> {
        let tier = self.plans.tier_of(owner_id, Utc::now()).await?;
        Ok(UploadLimits::for_tier(&tier))
    }

    pub async fn execute(
        &self,
        owner_id: UserId,
        file_name: String,
        content_type: String,
        declared_bytes: u64,
        limits: UploadLimits,
        stream: ByteStream,
    ) -> Result<Attachment, UploadAttachmentError> {
        if declared_bytes == 0 {
            return Err(UploadAttachmentError::Empty);
        }
        if declared_bytes > limits.max_request_bytes() {
            return Err(UploadAttachmentError::TooLarge);
        }
        if !content_type_allowed(&content_type) {
            return Err(UploadAttachmentError::TypeNotAllowed);
        }

        let id = Uuid::new_v4();
        let storage_key = format!("attachments/{id}");
        let mut attachment = Attachment {
            id,
            owner_id: owner_id.uuid(),
            file_name,
            content_type,
            size_bytes: declared_bytes.min(limits.max_file_bytes) as i64,
            storage_key,
            created_at: Utc::now(),
        };

        let reserved = self
            .repository
            .insert_reserving_quota(
                &attachment,
                MAX_UNLINKED_COUNT,
                limits.max_unlinked_bytes as i64,
            )
            .await?;
        if !reserved {
            return Err(UploadAttachmentError::QuotaExceeded);
        }

        let too_large = Arc::new(AtomicBool::new(false));
        let seen = Arc::new(AtomicU64::new(0));
        let limited: ByteStream = {
            let too_large = too_large.clone();
            let seen = seen.clone();
            let max = limits.max_file_bytes;
            Box::pin(stream.map(move |chunk| {
                let chunk = chunk?;
                let total = seen.fetch_add(chunk.len() as u64, Ordering::Relaxed) + chunk.len() as u64;
                if total > max {
                    too_large.store(true, Ordering::Relaxed);
                    return Err(FileStorageError::Internal("file exceeds the plan limit".to_string()));
                }
                Ok(chunk)
            }))
        };

        let written = self
            .storage
            .put_stream(&attachment.storage_key, &attachment.content_type, limited)
            .await;
        let failure = match written {
            Ok(0) => Some(UploadAttachmentError::Empty),
            Ok(bytes) => match self.repository.set_size(attachment.id, bytes as i64).await {
                Ok(()) => {
                    attachment.size_bytes = bytes as i64;
                    None
                }
                Err(e) => Some(e.into()),
            },
            Err(_) if too_large.load(Ordering::Relaxed) => Some(UploadAttachmentError::TooLarge),
            Err(e) => Some(e.into()),
        };
        let Some(failure) = failure else {
            return Ok(attachment);
        };

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
        Err(failure)
    }
}

#[cfg(test)]
mod tests {
    use super::UploadLimits;

    const MEGABYTE: u64 = 1024 * 1024;

    #[test]
    fn a_plan_sets_the_largest_file() {
        assert_eq!(UploadLimits::for_tier("free").max_file_bytes, 15 * MEGABYTE);
        assert_eq!(UploadLimits::for_tier("pro").max_file_bytes, 100 * MEGABYTE);
        assert_eq!(UploadLimits::for_tier("premium").max_file_bytes, 500 * MEGABYTE);
        assert_eq!(UploadLimits::for_tier("gold").max_file_bytes, 15 * MEGABYTE);
    }

    #[test]
    fn pending_uploads_leave_room_for_a_full_message() {
        assert_eq!(UploadLimits::for_tier("free").max_unlinked_bytes, 200 * MEGABYTE);
        assert_eq!(UploadLimits::for_tier("pro").max_unlinked_bytes, 1000 * MEGABYTE);
        assert_eq!(UploadLimits::for_tier("premium").max_unlinked_bytes, 10_000 * MEGABYTE);
    }

    #[test]
    fn a_request_may_carry_a_little_more_than_the_file() {
        let limits = UploadLimits::for_tier("free");
        assert!(limits.max_request_bytes() > limits.max_file_bytes);
        assert!(limits.max_request_bytes() < limits.max_file_bytes + MEGABYTE);
    }
}
