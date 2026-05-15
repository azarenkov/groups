use std::sync::Arc;

use domain::{
    server::{
        Server, description::ServerDescription, name::ServerName, repository::ServerRepository,
    },
    shared::{
        outbox::{
            outbox_record::OutboxRecord,
            to_outbox::{OutboxError, ToOutboxRecord},
        },
        repository::error::RepositoryError,
    },
    user::id::UserId,
};

use crate::dto::requests::server::create_server::CreateServerRequest;

#[derive(Debug, thiserror::Error)]
pub enum CreateServerError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
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
        let mut server = Server::create(
            ServerName::new(request.name),
            owner_id,
            request.description.map(ServerDescription::new),
        );

        let request_id = shared::middleware::request_id::current();
        let records: Vec<OutboxRecord> = server
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
            .collect::<Result<_, _>>()?;

        self.server_repository
            .save_server_with_events(&server, &records)
            .await?;

        Ok(server)
    }
}
