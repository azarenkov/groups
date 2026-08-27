use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{events::MessagingEvent, server::description::ServerDescription, server::name::ServerName, server::policy::PolicyError, server::unit_of_work::ServerUnitOfWork};
use ports::repository::RepositoryError;

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
            RepositoryError::NotFound => UpdateServerError::ServerNotFound,
            other => UpdateServerError::Repository(other),
        }
    }
}

impl From<PolicyError> for UpdateServerError {
    fn from(_: PolicyError) -> Self {
        UpdateServerError::NotOwner
    }
}

pub struct UpdateServer<W: ServerUnitOfWork> {
    unit_of_work: Arc<W>,
}

impl<W: ServerUnitOfWork> UpdateServer<W> {
    pub fn new(unit_of_work: Arc<W>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(&self, request: UpdateServerRequest) -> Result<(), UpdateServerError> {
        let UpdateServerRequest {
            server_id,
            owner_id,
            name,
            description,
            ctx,
        } = request;

        let mut tx = self.unit_of_work.begin(&server_id).await?;
        let mut server = tx.load_server().await?;
        server.update_details(
            &owner_id,
            ServerName::new(name.clone()),
            description.clone().map(ServerDescription::new),
            Utc::now(),
        )?;
        let events: Vec<MessagingEvent> = server.pull_events().into_iter().map(MessagingEvent::from).collect();

        if !tx.update_server_details(&name, description.as_deref()).await? {
            return Err(UpdateServerError::ServerNotFound);
        }
        tx.save_events(&events, ctx.request_id.as_deref()).await?;
        tx.commit().await?;
        Ok(())
    }
}
