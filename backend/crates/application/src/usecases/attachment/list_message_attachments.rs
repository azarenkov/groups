use std::sync::Arc;

use domain::{
    attachment::{Attachment, repository::AttachmentRepository},
    shared::repository::error::RepositoryError,
};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum ListMessageAttachmentsError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct ListMessageAttachments<R: AttachmentRepository> {
    repository: Arc<R>,
}

impl<R: AttachmentRepository> ListMessageAttachments<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    pub async fn for_channel_messages(
        &self,
        message_ids: &[Uuid],
    ) -> Result<Vec<Attachment>, ListMessageAttachmentsError> {
        if message_ids.is_empty() {
            return Ok(Vec::new());
        }
        Ok(self.repository.list_for_messages(message_ids).await?)
    }

    pub async fn for_dm_messages(
        &self,
        dm_message_ids: &[Uuid],
    ) -> Result<Vec<Attachment>, ListMessageAttachmentsError> {
        if dm_message_ids.is_empty() {
            return Ok(Vec::new());
        }
        Ok(self.repository.list_for_dm_messages(dm_message_ids).await?)
    }
}
