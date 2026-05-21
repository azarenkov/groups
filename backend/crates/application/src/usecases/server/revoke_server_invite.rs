use std::sync::Arc;

use domain::{
    server::{id::ServerId, repository::ServerRepository},
    shared::repository::error::RepositoryError,
    user::id::UserId,
};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum RevokeServerInviteError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Not the server owner")]
    NotOwner,
    #[error("Invite not found")]
    NotFound,
}

pub struct RevokeServerInvite<T>
where
    T: ServerRepository,
{
    server_repository: Arc<T>,
}

impl<T> RevokeServerInvite<T>
where
    T: ServerRepository,
{
    pub fn new(server_repository: Arc<T>) -> Self {
        Self { server_repository }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        invite_id: Uuid,
        requester_id: UserId,
    ) -> Result<(), RevokeServerInviteError> {
        let owner = self
            .server_repository
            .find_server_owner(&server_id)
            .await?
            .ok_or(RevokeServerInviteError::NotOwner)?;
        if owner != requester_id {
            return Err(RevokeServerInviteError::NotOwner);
        }

        let revoked = self
            .server_repository
            .revoke_invite(&server_id, &invite_id)
            .await?;
        if !revoked {
            return Err(RevokeServerInviteError::NotFound);
        }
        Ok(())
    }
}
