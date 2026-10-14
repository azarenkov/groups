use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use messaging_domain::{
    community::{
        events::{CommunityEvent, lifecycle::CommunityMemberJoined},
        invitation::{InvitationError, InvitationId, InvitationRepository, InvitationResolution},
        unit_of_work::CommunityUnitOfWork,
    },
    events::MessagingEvent,
    message::system::SystemMessageKind,
};
use ports::repository::RepositoryError;
use uuid::Uuid;

use crate::dto::RequestContext;
use crate::usecases::community::membership_announcements::announce_membership;

#[derive(Debug, thiserror::Error)]
pub enum AcceptCommunityInviteError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("This invite no longer exists")]
    NotFound,
    #[error("This invite was not addressed to you")]
    NotRecipient,
    #[error("This user is blocked")]
    Blocked,
}

pub struct AcceptCommunityInvite<I, W>
where
    I: InvitationRepository,
    W: CommunityUnitOfWork,
{
    invitations: Arc<I>,
    unit_of_work: Arc<W>,
}

impl<I, W> AcceptCommunityInvite<I, W>
where
    I: InvitationRepository,
    W: CommunityUnitOfWork,
{
    pub fn new(invitations: Arc<I>, unit_of_work: Arc<W>) -> Self {
        Self { invitations, unit_of_work }
    }

    pub async fn execute(
        &self,
        invitation_id: InvitationId,
        requester_id: UserId,
        ctx: RequestContext,
    ) -> Result<Uuid, AcceptCommunityInviteError> {
        let invitation = self.invitations.find(&invitation_id).await?.ok_or(AcceptCommunityInviteError::NotFound)?;
        let now = Utc::now();
        let resolved = invitation
            .resolve(&requester_id, InvitationResolution::Accepted, now)
            .map_err(|e| match e {
                InvitationError::NotInvitee => AcceptCommunityInviteError::NotRecipient,
                _ => AcceptCommunityInviteError::NotFound,
            })?;
        let community_id = *invitation.community_id();
        let mut tx = self.unit_of_work.begin(&community_id).await?;
        tx.lock_pair(invitation.inviter_id(), &requester_id).await?;
        if tx.block_relation(invitation.inviter_id(), &requester_id).await?.any() {
            return Err(AcceptCommunityInviteError::Blocked);
        }
        let joined = match tx.join_by_invitation(&invitation_id, &requester_id).await {
            Ok(joined) => joined,
            Err(RepositoryError::NotFound) => return Err(AcceptCommunityInviteError::NotFound),
            Err(e) => return Err(e.into()),
        };
        let mut events = vec![MessagingEvent::from(resolved)];
        if joined {
            events.push(MessagingEvent::from(CommunityEvent::MemberJoined(CommunityMemberJoined::record(
                community_id,
                requester_id,
                now,
            ))));
            events.extend(announce_membership(&mut *tx, requester_id, SystemMessageKind::MemberJoined, now).await?);
        }
        tx.save_events(&events, ctx.request_id.as_deref()).await?;
        tx.commit().await?;
        Ok(community_id.uuid())
    }
}
