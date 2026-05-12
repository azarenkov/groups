use std::sync::Arc;

use chrono::Utc;
use domain::{
    server::{
        events::{ServerEvent, lifecycle::ServerDeleted},
        id::ServerId,
        repository::ServerRepository,
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

#[derive(Debug, thiserror::Error)]
pub enum DeleteServerError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
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
        let event = ServerEvent::Deleted(ServerDeleted::record(server_id, owner_id, Utc::now()));
        let records: Vec<OutboxRecord> = vec![event.to_outbox_record()?];

        let deleted = self
            .server_repository
            .delete_server_by_owner_with_events(&server_id, &owner_id, &records)
            .await?;

        Ok(deleted)
    }
}
