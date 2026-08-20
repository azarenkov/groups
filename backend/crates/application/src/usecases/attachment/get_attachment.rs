use std::sync::Arc;

use domain::{
    attachment::AttachmentLink, message::repository::MessageRepository,
    shared::repository::error::RepositoryError, user::id::UserId,
};
use uuid::Uuid;

use crate::ports::{
    attachment_claims::{AttachmentClaimError, AttachmentClaims},
    file_storage::{ByteStream, FileStorage, FileStorageError},
};

#[derive(Debug, thiserror::Error)]
pub enum GetAttachmentError {
    #[error("Attachment not found")]
    NotFound,
    #[error("Storage error: {0}")]
    Storage(#[from] FileStorageError),
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Attachment service unavailable: {0}")]
    Claims(#[from] AttachmentClaimError),
}

pub struct GetAttachment<M: MessageRepository, S: FileStorage, C: AttachmentClaims> {
    message_repository: Arc<M>,
    storage: Arc<S>,
    attachment_claims: Arc<C>,
}

impl<M: MessageRepository, S: FileStorage, C: AttachmentClaims> GetAttachment<M, S, C> {
    pub fn new(
        message_repository: Arc<M>,
        storage: Arc<S>,
        attachment_claims: Arc<C>,
    ) -> Self {
        Self {
            message_repository,
            storage,
            attachment_claims,
        }
    }

    pub async fn execute(
        &self,
        attachment_id: Uuid,
        user_id: UserId,
    ) -> Result<(AttachmentLink, ByteStream), GetAttachmentError> {
        let link = match self
            .message_repository
            .find_attachment_for_user(attachment_id, &user_id)
            .await?
        {
            Some(link) => link,
            None => self
                .attachment_claims
                .lookup_unclaimed(attachment_id, &user_id)
                .await?
                .ok_or(GetAttachmentError::NotFound)?,
        };

        let stream = self.storage.get_stream(&link.storage_key).await?;
        Ok((link, stream))
    }
}
