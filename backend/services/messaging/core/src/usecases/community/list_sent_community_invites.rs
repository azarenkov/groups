use std::sync::Arc;

use domain::user::id::UserId;
use messaging_domain::community::{
    id::CommunityId, invitation::InvitationRepository, policy::PolicyError, unit_of_work::CommunityUnitOfWork,
};
use ports::repository::RepositoryError;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum ListSentCommunityInvitesError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("You are not a member of this community")]
    NotMember,
    #[error("Missing permission to manage invites")]
    NotAllowed,
}

impl From<PolicyError> for ListSentCommunityInvitesError {
    fn from(err: PolicyError) -> Self {
        match err {
            PolicyError::NotMember => Self::NotMember,
            _ => Self::NotAllowed,
        }
    }
}

pub struct ListSentCommunityInvites<I, W>
where
    I: InvitationRepository,
    W: CommunityUnitOfWork,
{
    invitations: Arc<I>,
    unit_of_work: Arc<W>,
}

impl<I, W> ListSentCommunityInvites<I, W>
where
    I: InvitationRepository,
    W: CommunityUnitOfWork,
{
    pub fn new(invitations: Arc<I>, unit_of_work: Arc<W>) -> Self {
        Self { invitations, unit_of_work }
    }

    pub async fn execute(&self, community_id: CommunityId, viewer_id: UserId) -> Result<Vec<Uuid>, ListSentCommunityInvitesError> {
        let mut tx = self.unit_of_work.begin(&community_id).await?;
        let community = match tx.load_community().await {
            Ok(community) => community,
            Err(RepositoryError::NotFound) => return Err(ListSentCommunityInvitesError::NotMember),
            Err(e) => return Err(e.into()),
        };
        community.ensure_can_manage_invites(&viewer_id)?;
        tx.commit().await?;
        let invitees = self.invitations.pending_invitees(&community_id).await?;
        Ok(invitees.into_iter().map(|u| u.uuid()).collect())
    }
}
