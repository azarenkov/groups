use std::sync::Arc;

use domain::{
    attachment::repository::AttachmentRepository,
    shared::repository::error::RepositoryError, user::id::UserId,
};
use uuid::Uuid;

use crate::ports::file_storage::FileStorage;

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
            .delete_unlinked(attachment_id, &owner_id)
            .await?
            .ok_or(DeleteAttachmentError::NotFound)?;
        if let Err(e) = self.storage.delete(&storage_key).await {
            tracing::warn!(error = %e, key = %storage_key, "Failed to delete attachment object");
        }
        Ok(())
    }
}
