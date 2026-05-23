use std::sync::Arc;

use domain::{
    dm_message::{id::DmMessageId, repository::DmMessageRepository},
    server::repository::ServerRepository,
    shared::repository::error::RepositoryError,
    user::id::UserId,
};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum AcceptServerInviteError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("This message is not a server invite")]
    NotAnInvite,
    #[error("This invite was not addressed to you")]
    NotRecipient,
}

pub struct AcceptServerInvite<M, S>
where
    M: DmMessageRepository,
    S: ServerRepository,
{
    dm_repository: Arc<M>,
    server_repository: Arc<S>,
}

impl<M, S> AcceptServerInvite<M, S>
where
    M: DmMessageRepository,
    S: ServerRepository,
{
    pub fn new(dm_repository: Arc<M>, server_repository: Arc<S>) -> Self {
        Self {
            dm_repository,
            server_repository,
        }
    }

    pub async fn execute(
        &self,
        message_id: DmMessageId,
        requester_id: UserId,
    ) -> Result<Uuid, AcceptServerInviteError> {
        let message = self.dm_repository.find_by_id(&message_id).await?;

        let server_id = message
            .invite_server_id()
            .ok_or(AcceptServerInviteError::NotAnInvite)?;

        if message.recipient_id() != &requester_id {
            return Err(AcceptServerInviteError::NotRecipient);
        }

        self.server_repository
            .accept_invite(&message_id, server_id, &requester_id)
            .await?;

        Ok(server_id.uuid())
    }
}
