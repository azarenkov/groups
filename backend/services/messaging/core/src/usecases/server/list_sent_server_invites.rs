use std::sync::Arc;

use messaging_domain::{dm_message::repository::DmMessageRepository, server::id::ServerId, server::repository::ServerRepository};
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum ListSentServerInvitesError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("You are not a member of this server")]
    NotMember,
}

pub struct ListSentServerInvites<M, S>
where
    M: DmMessageRepository,
    S: ServerRepository,
{
    dm_repository: Arc<M>,
    server_repository: Arc<S>,
}

impl<M, S> ListSentServerInvites<M, S>
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
        server_id: ServerId,
        sender_id: UserId,
    ) -> Result<Vec<Uuid>, ListSentServerInvitesError> {
        if !self
            .server_repository
            .is_server_member(&server_id, &sender_id)
            .await?
        {
            return Err(ListSentServerInvitesError::NotMember);
        }

        let recipients = self
            .dm_repository
            .find_invite_recipients(&sender_id, &server_id)
            .await?;
        Ok(recipients.into_iter().map(|u| u.uuid()).collect())
    }
}
