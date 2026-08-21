use std::sync::Arc;

use chrono::Utc;
use domain::{
    dm_message::{id::DmMessageId, repository::DmMessageRepository},
    shared::{
        outbox::{
            outbox_record::OutboxRecord,
            to_outbox::{OutboxError, ToOutboxRecord},
        },
        repository::error::RepositoryError,
    },
    user::id::UserId,
};

use crate::dto::RequestContext;

#[derive(Debug, thiserror::Error)]
pub enum DeclineServerInviteError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
    #[error("This message is not a server invite")]
    NotAnInvite,
    #[error("This invite was not addressed to you")]
    NotRecipient,
}

pub struct DeclineServerInvite<M>
where
    M: DmMessageRepository,
{
    dm_repository: Arc<M>,
}

impl<M> DeclineServerInvite<M>
where
    M: DmMessageRepository,
{
    pub fn new(dm_repository: Arc<M>) -> Self {
        Self { dm_repository }
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
        let records: Vec<OutboxRecord> = message
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
            .collect::<Result<_, _>>()?;

        self.dm_repository
            .delete_with_events(&message_id, &records)
            .await?;
        Ok(())
    }
}
