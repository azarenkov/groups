use std::sync::Arc;

use messaging_domain::{server::Server, server::repository::ServerRepository};
use domain::{pagination::PaginatedResult, pagination::Pagination, user::id::UserId};
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum ListServersByUserError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct ListServersByUser<T>
where
    T: ServerRepository,
{
    server_repository: Arc<T>,
}

impl<T> ListServersByUser<T>
where
    T: ServerRepository,
{
    pub fn new(server_repository: Arc<T>) -> Self {
        Self { server_repository }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        pagination: Pagination,
    ) -> Result<PaginatedResult<Server>, ListServersByUserError> {
        let result = self
            .server_repository
            .find_servers_by_user_id(&user_id, pagination)
            .await?;

        Ok(result)
    }
}
