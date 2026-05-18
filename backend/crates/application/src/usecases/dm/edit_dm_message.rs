use std::sync::Arc;

use chrono::Utc;
use domain::{
    dm_message::{DmMessageError, id::DmMessageId, repository::DmMessageRepository},
    message::{MessageError, content::MessageContent},
    shared::repository::error::RepositoryError,
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

        let new_content = MessageContent::new(request.content)?;
        let now = Utc::now();
        message.edit(new_content, editor_id, now)?;
        self.dm_repository.update(&message).await?;
        Ok(())
    }
}
