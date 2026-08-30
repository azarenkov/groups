use std::sync::Arc;

use messaging_domain::{events::MessagingEvent, server::Server, server::description::ServerDescription, server::name::ServerName, server::unit_of_work::ServerUnitOfWork};
use domain::user::id::UserId;
use ports::repository::RepositoryError;

use crate::dto::requests::server::create_server::CreateServerRequest;

#[derive(Debug, thiserror::Error)]
pub enum CreateServerError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Account is inactive")]
    AccountInactive,
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
        let events = server.pull_events();

        let events: Vec<MessagingEvent> = events.into_iter().map(MessagingEvent::from).collect();
        let mut tx = self.unit_of_work.begin(server.id()).await?;
        tx.lock_user(server.owner_id()).await?;
        if tx.is_user_deleted(server.owner_id()).await? {
            return Err(CreateServerError::AccountInactive);
        }
        tx.insert_server(&server).await?;
        tx.save_events(&events, request_id.as_deref()).await?;
        tx.commit().await?;

        Ok(server)
    }
}
