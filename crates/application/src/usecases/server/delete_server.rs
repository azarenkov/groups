use std::sync::Arc;

use domain::{
    repositories::{error::RepositoryError, server::ServerRepository},
    value_objects::{server::id::ServerId, user::id::UserId},
};

#[derive(Debug, thiserror::Error)]
pub enum DeleteServerError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct DeleteServer<T>
where
    T: ServerRepository,
{
    server_repository: Arc<T>,
}

impl<T> DeleteServer<T>
where
    T: ServerRepository,
{
    pub fn new(server_repository: Arc<T>) -> Self {
        Self { server_repository }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        owner_id: UserId,
    ) -> Result<bool, DeleteServerError> {
        Ok(self
            .server_repository
            .delete_server_by_owner(&server_id, &owner_id)
            .await?)
    }
}
