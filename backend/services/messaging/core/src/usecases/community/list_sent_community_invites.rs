use std::sync::Arc;

use domain::user::id::UserId;
use messaging_domain::community::{id::CommunityId, invitation::InvitationRepository, repository::CommunityRepository};
use ports::repository::RepositoryError;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum ListSentCommunityInvitesError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("You are not a member of this community")]
    NotMember,
}

pub struct ListSentCommunityInvites<I, S>
where
    I: InvitationRepository,
    S: CommunityRepository,
{
    invitations: Arc<I>,
    community_repository: Arc<S>,
}

impl<I, S> ListSentCommunityInvites<I, S>
where
    I: InvitationRepository,
    S: CommunityRepository,
{
    pub fn new(invitations: Arc<I>, community_repository: Arc<S>) -> Self {
        Self { invitations, community_repository }
    }

    pub async fn execute(&self, community_id: CommunityId, viewer_id: UserId) -> Result<Vec<Uuid>, ListSentCommunityInvitesError> {
        if !self.community_repository.is_community_member(&community_id, &viewer_id).await? {
            return Err(ListSentCommunityInvitesError::NotMember);
        }
        let invitees = self.invitations.pending_invitees(&community_id).await?;
        Ok(invitees.into_iter().map(|u| u.uuid()).collect())
    }
}
