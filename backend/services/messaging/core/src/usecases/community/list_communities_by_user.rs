use std::sync::Arc;

use messaging_domain::{community::Community, community::repository::CommunityRepository};
use domain::{pagination::PaginatedResult, pagination::Pagination, user::id::UserId};
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum ListCommunitiesByUserError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct ListCommunitiesByUser<T>
where
    T: CommunityRepository,
{
    community_repository: Arc<T>,
}

impl<T> ListCommunitiesByUser<T>
where
    T: CommunityRepository,
{
    pub fn new(community_repository: Arc<T>) -> Self {
        Self { community_repository }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        pagination: Pagination,
    ) -> Result<PaginatedResult<Community>, ListCommunitiesByUserError> {
        let result = self
            .community_repository
            .find_communities_by_user_id(&user_id, pagination)
            .await?;

        Ok(result)
    }
}
