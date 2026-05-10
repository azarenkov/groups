use std::sync::Arc;

use domain::{
    channel::Channel,
    server::{id::ServerId, repository::ServerRepository},
    shared::{
        pagination::{PaginatedResult, Pagination},
        repository::error::RepositoryError,
    },
};

#[derive(Debug, thiserror::Error)]
pub enum ListServerChannelsError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
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
        pagination: Pagination,
    ) -> Result<PaginatedResult<Channel>, ListServerChannelsError> {
        let result = self
            .server_repository
            .find_rooms_by_server_id(&server_id, pagination)
            .await?;

        Ok(result)
    }
}
