use std::sync::Arc;

use domain::{
    server::{id::ServerId, repository::ServerRepository},
    shared::repository::error::RepositoryError,
    user::id::UserId,
};

use crate::dto::responses::server::server_invite::ServerInviteDto;

#[derive(Debug, thiserror::Error)]
pub enum ListServerInvitesError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Not the server owner")]
    NotOwner,
}

pub struct ListServerInvites<T>
where
    T: ServerRepository,
{
    server_repository: Arc<T>,
}

impl<T> ListServerInvites<T>
where
    T: ServerRepository,
{
    pub fn new(server_repository: Arc<T>) -> Self {
        Self { server_repository }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        requester_id: UserId,
    ) -> Result<Vec<ServerInviteDto>, ListServerInvitesError> {
        let owner = self
            .server_repository
            .find_server_owner(&server_id)
            .await?
            .ok_or(ListServerInvitesError::NotOwner)?;
        if owner != requester_id {
            return Err(ListServerInvitesError::NotOwner);
        }

        let invites = self
            .server_repository
            .find_invites_by_server(&server_id)
            .await?;
        Ok(invites.into_iter().map(ServerInviteDto::from).collect())
    }
}
