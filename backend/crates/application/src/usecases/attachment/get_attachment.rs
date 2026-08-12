use std::sync::Arc;

use domain::{
    attachment::{Attachment, repository::AttachmentRepository},
    shared::repository::error::RepositoryError,
    user::id::UserId,
};
use uuid::Uuid;

use crate::ports::file_storage::{ByteStream, FileStorage, FileStorageError};

#[derive(Debug, thiserror::Error)]
pub enum GetAttachmentError {
    #[error("Attachment not found")]
    NotFound,
    #[error("You do not have access to this attachment")]
    Forbidden,
    #[error("Storage error: {0}")]
    Storage(#[from] FileStorageError),
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct GetAttachment<R: AttachmentRepository, S: FileStorage> {
    repository: Arc<R>,
    storage: Arc<S>,
}

impl<R: AttachmentRepository, S: FileStorage> GetAttachment<R, S> {
    pub fn new(repository: Arc<R>, storage: Arc<S>) -> Self {
        Self {
            repository,
            storage,
        }
    }

    pub async fn execute(
        &self,
        attachment_id: Uuid,
        user_id: UserId,
    ) -> Result<(Attachment, ByteStream), GetAttachmentError> {
        let attachment = self
            .repository
            .find_by_id(attachment_id)
            .await?
            .ok_or(GetAttachmentError::NotFound)?;

        let allowed = self.repository.can_access(attachment_id, &user_id).await?;
        if !allowed {
            return Err(GetAttachmentError::Forbidden);
        }

        let stream = self.storage.get_stream(&attachment.storage_key).await?;
        Ok((attachment, stream))
    }
}
