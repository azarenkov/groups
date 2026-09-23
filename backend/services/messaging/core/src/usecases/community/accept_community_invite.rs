use messaging_domain::message::system::SystemMessageKind;

use crate::usecases::community::membership_announcements::announce_membership;
use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{events::MessagingEvent, dm_message::id::DmMessageId, dm_message::repository::DmMessageRepository, community::unit_of_work::CommunityUnitOfWork};
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug, thiserror::Error)]
pub enum AcceptCommunityInviteError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("This message is not a community invite")]
    NotAnInvite,
    #[error("This invite was not addressed to you")]
    NotRecipient,
    #[error("This user is blocked")]
    Blocked,
}

pub struct AcceptCommunityInvite<M, W>
where
    M: DmMessageRepository,
    W: CommunityUnitOfWork,
{
    dm_repository: Arc<M>,
    unit_of_work: Arc<W>,
}

impl<M, W> AcceptCommunityInvite<M, W>
where
    M: DmMessageRepository,
    W: CommunityUnitOfWork,
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
    ) -> Result<Uuid, AcceptCommunityInviteError> {
        let mut message = self.dm_repository.find_by_id(&message_id).await?;

        let community_id = message
            .invite_community_id()
            .copied()
            .ok_or(AcceptCommunityInviteError::NotAnInvite)?;

        if message.recipient_id() != &requester_id {
            return Err(AcceptCommunityInviteError::NotRecipient);
        }
        let sender_id = *message.sender_id();

        message.record_invite_deleted(Utc::now());

        let request_id = ctx.request_id.clone();
        let mut events: Vec<MessagingEvent> =
            message.pull_events().into_iter().map(MessagingEvent::from).collect();

        let mut tx = self.unit_of_work.begin(&community_id).await?;
        tx.lock_pair(&sender_id, &requester_id).await?;
        if tx.block_relation(&sender_id, &requester_id).await?.any() {
            return Err(AcceptCommunityInviteError::Blocked);
        }
        let joined = tx.accept_invite(&message_id, &requester_id).await?;
        if joined {
            let now = Utc::now();
            events.push(MessagingEvent::from(
                messaging_domain::community::events::CommunityEvent::MemberJoined(
                    messaging_domain::community::events::lifecycle::CommunityMemberJoined::record(
                        community_id,
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

        Ok(community_id.uuid())
    }
}
