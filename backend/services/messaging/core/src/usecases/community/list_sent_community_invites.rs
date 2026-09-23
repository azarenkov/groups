use std::sync::Arc;

use messaging_domain::{dm_message::repository::DmMessageRepository, community::id::CommunityId, community::repository::CommunityRepository};
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum ListSentCommunityInvitesError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("You are not a member of this community")]
    NotMember,
}

pub struct ListSentCommunityInvites<M, S>
where
    M: DmMessageRepository,
    S: CommunityRepository,
{
    dm_repository: Arc<M>,
    community_repository: Arc<S>,
}

impl<M, S> ListSentCommunityInvites<M, S>
where
    M: DmMessageRepository,
    S: CommunityRepository,
{
    pub fn new(dm_repository: Arc<M>, community_repository: Arc<S>) -> Self {
        Self {
            dm_repository,
            community_repository,
        }
    }

    pub async fn execute(
        &self,
        community_id: CommunityId,
        sender_id: UserId,
    ) -> Result<Vec<Uuid>, ListSentCommunityInvitesError> {
        if !self
            .community_repository
            .is_community_member(&community_id, &sender_id)
            .await?
        {
            return Err(ListSentCommunityInvitesError::NotMember);
        }

        let recipients = self
            .dm_repository
            .find_invite_recipients(&sender_id, &community_id)
            .await?;
        Ok(recipients.into_iter().map(|u| u.uuid()).collect())
    }
}
