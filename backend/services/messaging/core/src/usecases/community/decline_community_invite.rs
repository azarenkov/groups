use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use messaging_domain::{
    community::{
        invitation::{InvitationError, InvitationId, InvitationRepository, InvitationResolution},
        unit_of_work::CommunityUnitOfWork,
    },
    events::MessagingEvent,
};
use ports::repository::RepositoryError;

use crate::dto::RequestContext;

#[derive(Debug, thiserror::Error)]
pub enum DeclineCommunityInviteError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("This invite no longer exists")]
    NotFound,
    #[error("This invite was not addressed to you")]
    NotRecipient,
}

pub struct DeclineCommunityInvite<I, W>
where
    I: InvitationRepository,
    W: CommunityUnitOfWork,
{
    invitations: Arc<I>,
    unit_of_work: Arc<W>,
}

impl<I, W> DeclineCommunityInvite<I, W>
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
    ) -> Result<(), DeclineCommunityInviteError> {
        let invitation = self.invitations.find(&invitation_id).await?.ok_or(DeclineCommunityInviteError::NotFound)?;
        let resolved = invitation
            .resolve(&requester_id, InvitationResolution::Declined, Utc::now())
            .map_err(|e| match e {
                InvitationError::NotInvitee => DeclineCommunityInviteError::NotRecipient,
                _ => DeclineCommunityInviteError::NotFound,
            })?;
        let mut tx = self.unit_of_work.begin(invitation.community_id()).await?;
        if !tx.delete_invitation(&invitation_id).await? {
            return Err(DeclineCommunityInviteError::NotFound);
        }
        tx.save_events(&[MessagingEvent::from(resolved)], ctx.request_id.as_deref()).await?;
        tx.commit().await?;
        Ok(())
    }
}
