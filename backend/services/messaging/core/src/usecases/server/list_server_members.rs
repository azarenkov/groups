use std::sync::Arc;

use messaging_domain::{server::id::ServerId, server::repository::ServerRepository};
use domain::{pagination::PaginatedResult, pagination::Pagination, user::id::UserId, user::name::UserName};
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum ListServerMembersError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Not a member of the server")]
    NotMember,
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
        requester_id: UserId,
        pagination: Pagination,
    ) -> Result<PaginatedResult<(UserId, UserName)>, ListServerMembersError> {
        let is_member = self
            .server_repository
            .is_server_member(&server_id, &requester_id)
            .await?;
        if !is_member {
            return Err(ListServerMembersError::NotMember);
        }

        let result = self
            .server_repository
            .find_users_by_server_id(&server_id, pagination)
            .await?;

        Ok(result)
    }
}
