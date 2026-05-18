use std::sync::Arc;

use domain::{
    dm_message::{DmMessageError, id::DmMessageId, repository::DmMessageRepository},
    shared::repository::error::RepositoryError,
    user::id::UserId,
};

use crate::dto::requests::dm::delete_dm_message::DeleteDmMessageRequest;

#[derive(Debug, thiserror::Error)]
pub enum DeleteDmMessageError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("DM error: {0}")]
    Dm(#[from] DmMessageError),
    #[error("Message not found")]
    NotFound,
}

pub struct DeleteDmMessage<M>
where
    M: DmMessageRepository,
{
    dm_repository: Arc<M>,
}

impl<M> DeleteDmMessage<M>
where
    M: DmMessageRepository,
{
    pub fn new(dm_repository: Arc<M>) -> Self {
        Self { dm_repository }
    }

    pub async fn execute(
        &self,
        request: DeleteDmMessageRequest,
        deleter_id: UserId,
    ) -> Result<(), DeleteDmMessageError> {
        let message_id = DmMessageId::new(request.message_id);

        let message = self
            .dm_repository
            .find_by_id(&message_id)
            .await
            .map_err(|e| match e {
                RepositoryError::NotFound => DeleteDmMessageError::NotFound,
                other => DeleteDmMessageError::Repository(other),
            })?;

        message.can_delete(deleter_id)?;
        self.dm_repository.delete(&message_id).await?;
        Ok(())
    }
}
