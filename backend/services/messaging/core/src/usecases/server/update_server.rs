use std::sync::Arc;

use chrono::Utc;
use domain::{
    server::{
        description::ServerDescription,
        events::{ServerEvent, lifecycle::ServerUpdated},
        name::ServerName,
        repository::ServerRepository,
        role::{PERM_MANAGE_SERVER, ServerRoleRepository, has_permission},
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
            other => UpdateServerError::Repository(other),
        }
    }
}

pub struct UpdateServer<S, R>
where
    S: ServerRepository,
    R: ServerRoleRepository,
{
    server_repository: Arc<S>,
    role_repository: Arc<R>,
}

impl<S, R> UpdateServer<S, R>
where
    S: ServerRepository,
    R: ServerRoleRepository,
{
    pub fn new(server_repository: Arc<S>, role_repository: Arc<R>) -> Self {
        Self {
            server_repository,
            role_repository,
        }
    }

    pub async fn execute(&self, request: UpdateServerRequest) -> Result<(), UpdateServerError> {
        let UpdateServerRequest {
            server_id,
            owner_id,
            name,
            description,
            ctx,
        } = request;

        let actual_owner = self
            .server_repository
            .find_server_owner(&server_id)
            .await?
            .ok_or(UpdateServerError::ServerNotFound)?;

        if actual_owner.uuid() != owner_id.uuid() {
            let perms = self
                .role_repository
                .user_permissions(&server_id, &owner_id)
                .await
                .map_err(UpdateServerError::Repository)?;
            if !has_permission(perms, PERM_MANAGE_SERVER) {
                return Err(UpdateServerError::NotOwner);
            }
        }

        let member_ids = self
            .server_repository
            .find_member_ids_by_server_id(&server_id)
            .await?;

        let event = ServerEvent::Updated(ServerUpdated::record(
            server_id,
            ServerName::new(name.clone()),
            description.clone().map(ServerDescription::new),
            member_ids,
            Utc::now(),
        ));
        let records: Vec<OutboxRecord> =
            vec![event.to_outbox_record_with_request_id(ctx.request_id)?];

        let delegated = if actual_owner.uuid() == owner_id.uuid() {
            None
        } else {
            Some(&owner_id)
        };
        match self
            .server_repository
            .update_server_details(
                &server_id,
                &name,
                description.as_deref(),
                delegated,
                &records,
            )
            .await?
        {
            domain::server::repository::UpdateServerOutcome::Updated => Ok(()),
            domain::server::repository::UpdateServerOutcome::NotFound => {
                Err(UpdateServerError::ServerNotFound)
            }
            domain::server::repository::UpdateServerOutcome::Denied => {
                Err(UpdateServerError::NotOwner)
            }
        }
    }
}
