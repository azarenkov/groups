use std::sync::Arc;

use domain::{
    dm_message::repository::DmMessageRepository,
    server::id::ServerId,
    shared::repository::error::RepositoryError,
    user::id::UserId,
};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum ListSentServerInvitesError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct ListSentServerInvites<M>
where
    M: DmMessageRepository,
{
    dm_repository: Arc<M>,
}

impl<M> ListSentServerInvites<M>
where
    M: DmMessageRepository,
{
    pub fn new(dm_repository: Arc<M>) -> Self {
        Self { dm_repository }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        sender_id: UserId,
    ) -> Result<Vec<Uuid>, ListSentServerInvitesError> {
        let recipients = self
            .dm_repository
            .find_invite_recipients(&sender_id, &server_id)
            .await?;
        Ok(recipients.into_iter().map(|u| u.uuid()).collect())
    }
}
