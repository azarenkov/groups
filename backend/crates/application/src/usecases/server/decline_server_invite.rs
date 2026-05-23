use std::sync::Arc;

use domain::{
    dm_message::{id::DmMessageId, repository::DmMessageRepository},
    shared::repository::error::RepositoryError,
    user::id::UserId,
};

#[derive(Debug, thiserror::Error)]
pub enum DeclineServerInviteError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("This message is not a server invite")]
    NotAnInvite,
    #[error("This invite was not addressed to you")]
    NotRecipient,
}

pub struct DeclineServerInvite<M>
where
    M: DmMessageRepository,
{
    dm_repository: Arc<M>,
}

impl<M> DeclineServerInvite<M>
where
    M: DmMessageRepository,
{
    pub fn new(dm_repository: Arc<M>) -> Self {
        Self { dm_repository }
    }

    pub async fn execute(
        &self,
        message_id: DmMessageId,
        requester_id: UserId,
    ) -> Result<(), DeclineServerInviteError> {
        let message = self.dm_repository.find_by_id(&message_id).await?;

        if message.invite_server_id().is_none() {
            return Err(DeclineServerInviteError::NotAnInvite);
        }

        if message.recipient_id() != &requester_id {
            return Err(DeclineServerInviteError::NotRecipient);
        }

        self.dm_repository.delete_with_events(&message_id, &[]).await?;
        Ok(())
    }
}
