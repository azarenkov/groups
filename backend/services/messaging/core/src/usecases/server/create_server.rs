use std::sync::Arc;

use messaging_domain::{server::Server, server::description::ServerDescription, server::name::ServerName, server::unit_of_work::ServerUnitOfWork};
use domain::user::id::UserId;
use ports::{outbox::record::OutboxRecord, outbox::to_outbox::OutboxError, outbox::to_outbox::ToOutboxRecord, repository::RepositoryError};

use crate::dto::requests::server::create_server::CreateServerRequest;

#[derive(Debug, thiserror::Error)]
pub enum CreateServerError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
}

pub struct CreateServer<W>
where
    W: ServerUnitOfWork,
{
    unit_of_work: Arc<W>,
}

impl<W> CreateServer<W>
where
    W: ServerUnitOfWork,
{
    pub fn new(unit_of_work: Arc<W>) -> Self {
        Self { unit_of_work }
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
            request.accent_color,
        );

        let request_id = request.ctx.request_id.clone();
        let records: Vec<OutboxRecord> = server
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
            .collect::<Result<_, _>>()?;

        let mut tx = self.unit_of_work.begin(server.id()).await?;
        tx.insert_server(&server).await?;
        tx.save_outbox(&records).await?;
        tx.commit().await?;

        Ok(server)
    }
}
