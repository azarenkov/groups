use std::sync::Arc;

use chrono::Utc;
use domain::{
    dm_message::{DmMessageError, id::DmMessageId, repository::DmMessageRepository},
    message::{MessageError, content::MessageContent},
    shared::{
        outbox::{
            outbox_record::OutboxRecord,
            to_outbox::{OutboxError, ToOutboxRecord},
        },
        repository::error::RepositoryError,
    },
    user::id::UserId,
};

use crate::dto::requests::dm::edit_dm_message::EditDmMessageRequest;

#[derive(Debug, thiserror::Error)]
pub enum EditDmMessageError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("DM error: {0}")]
    Dm(#[from] DmMessageError),
    #[error("Message error: {0}")]
    Message(#[from] MessageError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
    #[error("Message not found")]
    NotFound,
}

pub struct EditDmMessage<M>
where
    M: DmMessageRepository,
{
    dm_repository: Arc<M>,
}

impl<M> EditDmMessage<M>
where
    M: DmMessageRepository,
{
    pub fn new(dm_repository: Arc<M>) -> Self {
        Self { dm_repository }
    }

    pub async fn execute(
        &self,
        request: EditDmMessageRequest,
        editor_id: UserId,
    ) -> Result<(), EditDmMessageError> {
        let message_id = DmMessageId::new(request.message_id);

        let mut message = self
            .dm_repository
            .find_by_id(&message_id)
            .await
            .map_err(|e| match e {
                RepositoryError::NotFound => EditDmMessageError::NotFound,
                other => EditDmMessageError::Repository(other),
            })?;

        if message.sender_id() != &editor_id {
            return Err(EditDmMessageError::NotFound);
        }

        let new_content = MessageContent::new(request.content)?;
        let now = Utc::now();
        message.edit(new_content, editor_id, now)?;
        let request_id = request.ctx.request_id.clone();
        let records: Vec<OutboxRecord> = message
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
            .collect::<Result<_, _>>()?;
        self.dm_repository
            .update_with_events(&message, &records)
            .await?;
        Ok(())
    }
}
