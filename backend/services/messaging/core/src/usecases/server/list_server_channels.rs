use std::sync::Arc;

use messaging_domain::{channel::Channel, server::id::ServerId, server::repository::ServerRepository};
use domain::{pagination::PaginatedResult, pagination::Pagination, user::id::UserId};
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum ListServerChannelsError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Not a member of the server")]
    NotMember,
}

pub struct ListServerChannels<T>
where
    T: ServerRepository,
{
    server_repository: Arc<T>,
}

impl<T> ListServerChannels<T>
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
        pagination: Pagination,
    ) -> Result<PaginatedResult<Channel>, ListServerChannelsError> {
        let is_member = self
            .server_repository
            .is_server_member(&server_id, &requester_id)
            .await?;
        if !is_member {
            return Err(ListServerChannelsError::NotMember);
        }

        let result = self
            .server_repository
            .find_channels_by_server_id(&server_id, pagination)
            .await?;

        Ok(result)
    }
}
