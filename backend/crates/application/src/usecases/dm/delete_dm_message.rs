use std::sync::Arc;

use chrono::Utc;
use domain::{
    attachment::repository::AttachmentRepository,
    dm_message::{DmMessageError, id::DmMessageId, repository::DmMessageRepository},
    shared::{
        outbox::{
            outbox_record::OutboxRecord,
            to_outbox::{OutboxError, ToOutboxRecord},
        },
        repository::error::RepositoryError,
    },
    user::id::UserId,
};

use crate::{
    dto::requests::dm::delete_dm_message::DeleteDmMessageRequest,
    ports::file_storage::FileStorage,
};

#[derive(Debug, thiserror::Error)]
pub enum DeleteDmMessageError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("DM error: {0}")]
    Dm(#[from] DmMessageError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
    #[error("Message not found")]
    NotFound,
}

pub struct DeleteDmMessage<M, A, S>
where
    M: DmMessageRepository,
    A: AttachmentRepository,
    S: FileStorage,
{
    dm_repository: Arc<M>,
    attachment_repository: Arc<A>,
    storage: Arc<S>,
}

impl<M, A, S> DeleteDmMessage<M, A, S>
where
    M: DmMessageRepository,
    A: AttachmentRepository,
    S: FileStorage,
{
    pub fn new(
        dm_repository: Arc<M>,
        attachment_repository: Arc<A>,
        storage: Arc<S>,
    ) -> Self {
        Self {
            dm_repository,
            attachment_repository,
            storage,
        }
    }

    pub async fn execute(
        &self,
        request: DeleteDmMessageRequest,
        deleter_id: UserId,
    ) -> Result<(), DeleteDmMessageError> {
        let message_id = DmMessageId::new(request.message_id);

        let mut message = self
            .dm_repository
            .find_by_id(&message_id)
            .await
            .map_err(|e| match e {
                RepositoryError::NotFound => DeleteDmMessageError::NotFound,
                other => DeleteDmMessageError::Repository(other),
            })?;

        if message.sender_id() != &deleter_id {
            return Err(DeleteDmMessageError::NotFound);
        }

        let now = Utc::now();
        message.mark_deleted(deleter_id, now)?;
        let request_id = request.ctx.request_id.clone();
        let records: Vec<OutboxRecord> = message
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
            .collect::<Result<_, _>>()?;
        let storage_keys = self
            .dm_repository
            .delete_with_events(&message_id, &records)
            .await?;
        for key in storage_keys {
            match self.storage.delete(&key).await {
                Ok(()) | Err(crate::ports::file_storage::FileStorageError::NotFound) => {
                    if let Err(e) = self.attachment_repository.dequeue_cleanup(&key).await {
                        tracing::warn!(error = %e, %key, "Failed to dequeue cleaned attachment");
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, %key, "Failed to delete attachment object; cleanup queue will retry");
                }
            }
        }
        Ok(())
    }
}
