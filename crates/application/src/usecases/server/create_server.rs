use std::sync::Arc;

use domain::{
    server::{
        Server,
        repository::ServerRepository,
        value_objects::{description::ServerDescription, name::ServerName},
    },
    shared::repository::error::RepositoryError,
    user::value_objects::id::UserId,
};

use crate::dto::requests::server::create_server::CreateServerRequest;

#[derive(Debug, thiserror::Error)]
pub enum CreateServerError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct CreateServer<T>
where
    T: ServerRepository,
{
    server_repository: Arc<T>,
}

impl<T> CreateServer<T>
where
    T: ServerRepository,
{
    pub fn new(server_repository: Arc<T>) -> Self {
        Self { server_repository }
    }

    pub async fn execute(
        &self,
        request: CreateServerRequest,
        owner_id: UserId,
    ) -> Result<Server, CreateServerError> {
        let server = Server::create(
            ServerName::new(request.name),
            owner_id,
            request.description.map(ServerDescription::new),
        );
        self.server_repository.create_server(&server).await?;

        Ok(server)
    }
}
