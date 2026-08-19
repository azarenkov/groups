use std::sync::Arc;

use domain::{
    attachment::AttachmentLink, message::repository::MessageRepository,
    shared::repository::error::RepositoryError, user::id::UserId,
};
use uuid::Uuid;

use crate::ports::file_storage::{ByteStream, FileStorage, FileStorageError};

#[derive(Debug, thiserror::Error)]
pub enum GetAttachmentError {
    #[error("Attachment not found")]
    NotFound,
    #[error("Storage error: {0}")]
    Storage(#[from] FileStorageError),
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct GetAttachment<M: MessageRepository, S: FileStorage> {
    message_repository: Arc<M>,
    storage: Arc<S>,
}

impl<M: MessageRepository, S: FileStorage> GetAttachment<M, S> {
    pub fn new(message_repository: Arc<M>, storage: Arc<S>) -> Self {
        Self {
            message_repository,
            storage,
        }
    }

    pub async fn execute(
        &self,
        attachment_id: Uuid,
        user_id: UserId,
    ) -> Result<(AttachmentLink, ByteStream), GetAttachmentError> {
        let link = self
            .message_repository
            .find_attachment_for_user(attachment_id, &user_id)
            .await?
            .ok_or(GetAttachmentError::NotFound)?;

        let stream = self.storage.get_stream(&link.storage_key).await?;
        Ok((link, stream))
    }
}
