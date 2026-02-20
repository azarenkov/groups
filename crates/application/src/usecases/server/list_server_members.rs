use std::sync::Arc;

use domain::{
    server::{repository::ServerRepository, value_objects::id::ServerId},
    shared::repository::error::RepositoryError,
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

    pub async fn execute(&self, server_id: ServerId) -> Result<Vec<User>, ListServerMembersError> {
        let members = self
            .server_repository
            .get_users_by_server_id(&server_id)
            .await?;

        Ok(members)
    }
}
