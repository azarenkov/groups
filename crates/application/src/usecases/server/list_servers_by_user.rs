use std::sync::Arc;

use domain::{
    server::{Server, repository::ServerRepository},
    shared::repository::error::RepositoryError,
    user::value_objects::id::UserId,
};

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

    pub async fn execute(&self, user_id: UserId) -> Result<Vec<Server>, ListServersByUserError> {
        let servers = self
            .server_repository
            .get_servers_by_user_id(&user_id)
            .await?;

        Ok(servers)
    }
}
