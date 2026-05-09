use std::sync::Arc;

use domain::{
    server::{repository::ServerRepository, id::ServerId},
    shared::{
        pagination::{PaginatedResult, Pagination},
        repository::error::RepositoryError,
    },
    user::User,
};

#[derive(Debug, thiserror::Error)]
pub enum ListServerMembersError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct ListServerMembers<T>
where
    T: ServerRepository,
{
    server_repository: Arc<T>,
}

impl<T> ListServerMembers<T>
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
    ) -> Result<PaginatedResult<User>, ListServerMembersError> {
        let result = self
            .server_repository
            .get_users_by_server_id(&server_id, pagination)
            .await?;

        Ok(result)
    }
}
