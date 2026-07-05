use std::sync::Arc;

use domain::{
    server::repository::ServerRepository, shared::repository::error::RepositoryError,
};

use crate::dto::requests::server::update_server::UpdateServerRequest;

#[derive(Debug, thiserror::Error)]
pub enum UpdateServerError {
    #[error("Server not found")]
    ServerNotFound,
    #[error("Only the server owner can edit it")]
    NotOwner,
    #[error("Server name already taken")]
    NameTaken,
    #[error("Repository error: {0}")]
    Repository(RepositoryError),
}

impl From<RepositoryError> for UpdateServerError {
    fn from(e: RepositoryError) -> Self {
        match e {
            RepositoryError::AlreadyExists => UpdateServerError::NameTaken,
            other => UpdateServerError::Repository(other),
        }
    }
}

pub struct UpdateServer<S>
where
    S: ServerRepository,
{
    server_repository: Arc<S>,
}

impl<S> UpdateServer<S>
where
    S: ServerRepository,
{
    pub fn new(server_repository: Arc<S>) -> Self {
        Self { server_repository }
    }

    pub async fn execute(&self, request: UpdateServerRequest) -> Result<(), UpdateServerError> {
        let UpdateServerRequest {
            server_id,
            owner_id,
            name,
            description,
            ..
        } = request;

        let actual_owner = self
            .server_repository
            .find_server_owner(&server_id)
            .await?
            .ok_or(UpdateServerError::ServerNotFound)?;

        if actual_owner.uuid() != owner_id.uuid() {
            return Err(UpdateServerError::NotOwner);
        }

        let updated = self
            .server_repository
            .update_server_details(&server_id, &name, description.as_deref())
            .await?;

        if !updated {
            return Err(UpdateServerError::ServerNotFound);
        }

        Ok(())
    }
}
