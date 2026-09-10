use messaging_domain::server::invite_link::InviteLink;

use crate::usecases::server::invite_links::create_with_fresh_token;
use std::sync::Arc;

use messaging_domain::{events::MessagingEvent, server::Server, server::description::ServerDescription, server::name::ServerName, server::unit_of_work::ServerUnitOfWork};
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use ports::text_moderation::{TextModeration, TextModerationError};

use crate::dto::requests::server::create_server::CreateServerRequest;
use crate::usecases::server::text_policy::{ServerTextField, ensure_server_text_allowed};

#[derive(Debug, thiserror::Error)]
pub enum CreateServerError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Account is inactive")]
    AccountInactive,
    #[error("Server name is not allowed")]
    NameNotAllowed,
    #[error("Server description is not allowed")]
    DescriptionNotAllowed,
    #[error("Text moderation unavailable: {0}")]
    ModerationUnavailable(String),
}

pub struct CreateServer<W, TM>
where
    W: ServerUnitOfWork,
    TM: TextModeration,
{
    unit_of_work: Arc<W>,
    text_moderation: Arc<TM>,
}

impl<W, TM> CreateServer<W, TM>
where
    W: ServerUnitOfWork,
    TM: TextModeration,
{
    pub fn new(unit_of_work: Arc<W>, text_moderation: Arc<TM>) -> Self {
        Self {
            unit_of_work,
            text_moderation,
        }
    }

    pub async fn execute(
        &self,
        request: CreateServerRequest,
        owner_id: UserId,
    ) -> Result<Server, CreateServerError> {
        match ensure_server_text_allowed(
            self.text_moderation.as_ref(),
            &request.name,
            request.description.as_deref(),
        )
        .await
        {
            Ok(()) => {}
            Err((ServerTextField::Name, TextModerationError::Rejected)) => {
                return Err(CreateServerError::NameNotAllowed);
            }
            Err((ServerTextField::Description, TextModerationError::Rejected)) => {
                return Err(CreateServerError::DescriptionNotAllowed);
            }
            Err((_, TextModerationError::Unavailable(e))) => {
                return Err(CreateServerError::ModerationUnavailable(e));
            }
        }
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
        create_with_fresh_token(&mut *tx, || InviteLink::primary(*server.id(), owner_id, chrono::Utc::now())).await?;
        tx.save_events(&events, request_id.as_deref()).await?;
        tx.commit().await?;

        Ok(server)
    }
}
