use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{events::MessagingEvent, dm_message::id::DmMessageId, dm_message::repository::DmMessageRepository, dm_message::unit_of_work::DmUnitOfWork};
use domain::user::id::UserId;
use ports::repository::RepositoryError;

use crate::dto::RequestContext;

#[derive(Debug, thiserror::Error)]
pub enum DeclineServerInviteError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("This message is not a server invite")]
    NotAnInvite,
    #[error("This invite was not addressed to you")]
    NotRecipient,
}

pub struct DeclineServerInvite<M, W>
where
    M: DmMessageRepository,
    W: DmUnitOfWork,
{
    dm_repository: Arc<M>,
    unit_of_work: Arc<W>,
}

impl<M, W> DeclineServerInvite<M, W>
where
    M: DmMessageRepository,
    W: DmUnitOfWork,
{
    pub fn new(dm_repository: Arc<M>, unit_of_work: Arc<W>) -> Self {
        Self {
            dm_repository,
            unit_of_work,
        }
    }

    pub async fn execute(
        &self,
        message_id: DmMessageId,
        requester_id: UserId,
        ctx: RequestContext,
    ) -> Result<(), DeclineServerInviteError> {
        let mut message = self.dm_repository.find_by_id(&message_id).await?;

        if message.invite_server_id().is_none() {
            return Err(DeclineServerInviteError::NotAnInvite);
        }

        if message.recipient_id() != &requester_id {
            return Err(DeclineServerInviteError::NotRecipient);
        }

        message.record_invite_deleted(Utc::now());

        let request_id = ctx.request_id.clone();
        let events = message.pull_events();

        let events: Vec<MessagingEvent> = events.into_iter().map(MessagingEvent::from).collect();
        let mut tx = self.unit_of_work.begin().await?;
        tx.delete_message(&message_id).await?;
        tx.save_events(&events, request_id.as_deref()).await?;
        tx.commit().await?;
        Ok(())
    }
}
