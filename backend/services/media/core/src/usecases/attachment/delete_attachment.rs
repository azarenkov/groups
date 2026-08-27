use std::sync::Arc;

use media_domain::attachment::repository::AttachmentRepository;
use domain::{shared::repository::error::RepositoryError, user::id::UserId};
use uuid::Uuid;

use crate::ports::file_storage::{FileStorage, FileStorageError};

#[derive(Debug, thiserror::Error)]
pub enum DeleteAttachmentError {
    #[error("Attachment not found")]
    NotFound,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct DeleteAttachment<R: AttachmentRepository, S: FileStorage> {
    repository: Arc<R>,
    storage: Arc<S>,
}

impl<R: AttachmentRepository, S: FileStorage> DeleteAttachment<R, S> {
    pub fn new(repository: Arc<R>, storage: Arc<S>) -> Self {
        Self {
            repository,
            storage,
        }
    }

    pub async fn execute(
        &self,
        attachment_id: Uuid,
        owner_id: UserId,
    ) -> Result<(), DeleteAttachmentError> {
        let storage_key = self
            .repository
            .claim_unlinked(attachment_id, &owner_id)
            .await?
            .ok_or(DeleteAttachmentError::NotFound)?;
        match self.storage.delete(&storage_key).await {
            Ok(()) | Err(FileStorageError::NotFound) => {
                self.repository.delete_orphan_row(attachment_id).await?;
            }
            Err(e) => {
                tracing::warn!(error = %e, key = %storage_key, "Failed to delete attachment object; sweeper will retry");
            }
        }
        Ok(())
    }
}
