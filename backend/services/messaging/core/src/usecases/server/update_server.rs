use std::sync::Arc;

use chrono::Utc;
use domain::{
    server::{
        description::ServerDescription,
        events::{ServerEvent, lifecycle::ServerUpdated},
        name::ServerName,
        policy::{ModerationPolicy, PolicyError},
        unit_of_work::ServerUnitOfWork,
    },
    shared::{
        outbox::{
            outbox_record::OutboxRecord,
            to_outbox::{OutboxError, ToOutboxRecord},
        },
        repository::error::RepositoryError,
    },
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
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
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
        let snapshot = tx.authorization().await?;
        ModerationPolicy::manage_server(&snapshot, &owner_id)?;

        let member_ids: Vec<_> = snapshot.members.iter().copied().collect();
        let event = ServerEvent::Updated(ServerUpdated::record(
            server_id,
            ServerName::new(name.clone()),
            description.clone().map(ServerDescription::new),
            member_ids,
            Utc::now(),
        ));
        let records: Vec<OutboxRecord> =
            vec![event.to_outbox_record_with_request_id(ctx.request_id)?];

        if !tx.update_server_details(&name, description.as_deref()).await? {
            return Err(UpdateServerError::ServerNotFound);
        }
        tx.save_outbox(&records).await?;
        tx.commit().await?;
        Ok(())
    }
}
