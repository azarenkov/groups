use std::sync::Arc;

use messaging_domain::{community::id::CommunityId, community::repository::CommunityRepository};
use domain::{pagination::PaginatedResult, pagination::Pagination, user::id::UserId, user::name::UserName};
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum ListCommunityMembersError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Not a member of the community")]
    NotMember,
}

pub struct ListCommunityMembers<T>
where
    T: CommunityRepository,
{
    community_repository: Arc<T>,
}

impl<T> ListCommunityMembers<T>
where
    T: CommunityRepository,
{
    pub fn new(community_repository: Arc<T>) -> Self {
        Self { community_repository }
    }

    pub async fn execute(
        &self,
        community_id: CommunityId,
        requester_id: UserId,
        pagination: Pagination,
    ) -> Result<PaginatedResult<(UserId, UserName)>, ListCommunityMembersError> {
        let is_member = self
            .community_repository
            .is_community_member(&community_id, &requester_id)
            .await?;
        if !is_member {
            return Err(ListCommunityMembersError::NotMember);
        }

        let result = self
            .community_repository
            .find_users_by_community_id(&community_id, pagination)
            .await?;

        Ok(result)
    }
}
