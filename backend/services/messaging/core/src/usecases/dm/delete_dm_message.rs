use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{dm_message::DmMessageError, dm_message::id::DmMessageId, dm_message::repository::DmMessageRepository, dm_message::unit_of_work::DmUnitOfWork};
use domain::user::id::UserId;
use ports::{outbox::record::OutboxRecord, outbox::to_outbox::OutboxError, outbox::to_outbox::ToOutboxRecord, repository::RepositoryError};

use crate::dto::requests::dm::delete_dm_message::DeleteDmMessageRequest;

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

pub struct DeleteDmMessage<M, W>
where
    M: DmMessageRepository,
    W: DmUnitOfWork,
{
    dm_repository: Arc<M>,
    unit_of_work: Arc<W>,
}

impl<M, W> DeleteDmMessage<M, W>
where
    M: DmMessageRepository,
    W: DmUnitOfWork,
{
    pub fn new(dm_repository: Arc<M>, unit_of_work: Arc<W>) -> Self {
        Self {
            dm_repository,
            unit_of_work,
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
        let mut tx = self.unit_of_work.begin().await?;
        tx.delete_message(&message_id).await.map_err(|e| match e {
            RepositoryError::NotFound => DeleteDmMessageError::NotFound,
            other => DeleteDmMessageError::Repository(other),
        })?;
        tx.save_outbox(&records).await?;
        tx.commit().await?;
        Ok(())
    }
}
