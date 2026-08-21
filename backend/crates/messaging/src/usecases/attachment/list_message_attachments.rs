use std::sync::Arc;

use domain::{
    attachment::AttachmentLink, dm_message::repository::DmMessageRepository,
    message::repository::MessageRepository, shared::repository::error::RepositoryError,
};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum ListMessageAttachmentsError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct ListMessageAttachments<M, D>
where
    M: MessageRepository,
    D: DmMessageRepository,
{
    message_repository: Arc<M>,
    dm_repository: Arc<D>,
}

impl<M, D> ListMessageAttachments<M, D>
where
    M: MessageRepository,
    D: DmMessageRepository,
{
    pub fn new(message_repository: Arc<M>, dm_repository: Arc<D>) -> Self {
        Self {
            message_repository,
            dm_repository,
        }
    }

    pub async fn for_messages(
        &self,
        message_ids: &[Uuid],
    ) -> Result<Vec<(Uuid, AttachmentLink)>, ListMessageAttachmentsError> {
        if message_ids.is_empty() {
            return Ok(Vec::new());
        }
        Ok(self
            .message_repository
            .list_attachment_links_for_messages(message_ids)
            .await?)
    }

    pub async fn for_dm_messages(
        &self,
        dm_message_ids: &[Uuid],
    ) -> Result<Vec<(Uuid, AttachmentLink)>, ListMessageAttachmentsError> {
        if dm_message_ids.is_empty() {
            return Ok(Vec::new());
        }
        Ok(self
            .dm_repository
            .list_attachment_links_for_dm_messages(dm_message_ids)
            .await?)
    }
}
