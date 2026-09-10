use messaging_domain::message::system::SystemMessageKind;

use crate::usecases::server::membership_announcements::announce_membership;
use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{events::MessagingEvent, dm_message::id::DmMessageId, dm_message::repository::DmMessageRepository, server::unit_of_work::ServerUnitOfWork};
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug, thiserror::Error)]
pub enum AcceptServerInviteError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("This message is not a server invite")]
    NotAnInvite,
    #[error("This invite was not addressed to you")]
    NotRecipient,
    #[error("This user is blocked")]
    Blocked,
}

pub struct AcceptServerInvite<M, W>
where
    M: DmMessageRepository,
    W: ServerUnitOfWork,
{
    dm_repository: Arc<M>,
    unit_of_work: Arc<W>,
}

impl<M, W> AcceptServerInvite<M, W>
where
    M: DmMessageRepository,
    W: ServerUnitOfWork,
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
    ) -> Result<Uuid, AcceptServerInviteError> {
        let mut message = self.dm_repository.find_by_id(&message_id).await?;

        let server_id = message
            .invite_server_id()
            .copied()
            .ok_or(AcceptServerInviteError::NotAnInvite)?;

        if message.recipient_id() != &requester_id {
            return Err(AcceptServerInviteError::NotRecipient);
        }
        let sender_id = *message.sender_id();

        message.record_invite_deleted(Utc::now());

        let request_id = ctx.request_id.clone();
        let mut events: Vec<MessagingEvent> =
            message.pull_events().into_iter().map(MessagingEvent::from).collect();

        let mut tx = self.unit_of_work.begin(&server_id).await?;
        tx.lock_pair(&sender_id, &requester_id).await?;
        if tx.block_relation(&sender_id, &requester_id).await?.any() {
            return Err(AcceptServerInviteError::Blocked);
        }
        let joined = tx.accept_invite(&message_id, &requester_id).await?;
        if joined {
            let now = Utc::now();
            events.push(MessagingEvent::from(
                messaging_domain::server::events::ServerEvent::MemberJoined(
                    messaging_domain::server::events::lifecycle::ServerMemberJoined::record(
                        server_id,
                        requester_id,
                        now,
                    ),
                ),
            ));
            events.extend(
                announce_membership(&mut *tx, requester_id, SystemMessageKind::MemberJoined, now)
                    .await?,
            );
        }
        tx.save_events(&events, request_id.as_deref()).await?;
        tx.commit().await?;

        Ok(server_id.uuid())
    }
}
