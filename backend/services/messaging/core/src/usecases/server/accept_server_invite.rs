use std::sync::Arc;

use chrono::Utc;
use domain::{
    dm_message::{id::DmMessageId, repository::DmMessageRepository},
    server::repository::ServerRepository,
    shared::{
        outbox::{
            outbox_record::OutboxRecord,
            to_outbox::{OutboxError, ToOutboxRecord},
        },
        repository::error::RepositoryError,
    },
    user::id::UserId,
};
use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug, thiserror::Error)]
pub enum AcceptServerInviteError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
    #[error("This message is not a server invite")]
    NotAnInvite,
    #[error("This invite was not addressed to you")]
    NotRecipient,
}

pub struct AcceptServerInvite<M, S>
where
    M: DmMessageRepository,
    S: ServerRepository,
{
    dm_repository: Arc<M>,
    server_repository: Arc<S>,
}

impl<M, S> AcceptServerInvite<M, S>
where
    M: DmMessageRepository,
    S: ServerRepository,
{
    pub fn new(dm_repository: Arc<M>, server_repository: Arc<S>) -> Self {
        Self {
            dm_repository,
            server_repository,
        }
    }

    pub async fn execute(
        &self,
        message_id: DmMessageId,
        requester_id: UserId,
        ctx: RequestContext,
    ) -> Result<Uuid, AcceptServerInviteError> {
        let mut message = self.dm_repository.find_by_id(&message_id).await?;

        let server_id = message
            .invite_server_id()
            .copied()
            .ok_or(AcceptServerInviteError::NotAnInvite)?;

        if message.recipient_id() != &requester_id {
            return Err(AcceptServerInviteError::NotRecipient);
        }

        message.record_invite_deleted(Utc::now());

        let request_id = ctx.request_id.clone();
        let mut records: Vec<OutboxRecord> = message
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
            .collect::<Result<_, _>>()?;
        let member_event = domain::server::events::ServerEvent::MemberJoined(
            domain::server::events::lifecycle::ServerMemberJoined::record(
                server_id,
                requester_id,
                Utc::now(),
            ),
        );
        records.push(member_event.to_outbox_record_with_request_id(request_id.clone())?);

        self.server_repository
            .accept_invite(&message_id, &server_id, &requester_id, &records)
            .await?;

        Ok(server_id.uuid())
    }
}
