use std::sync::Arc;

use domain::{
    dm_message::repository::DmMessageRepository,
    friendship::{repository::FriendshipRepository, status::FriendshipStatus},
    shared::{pagination::PaginatedResult, repository::error::RepositoryError},
    user::id::UserId,
};

use crate::dto::{
    requests::dm::search_dm_messages::SearchDmMessagesRequest,
    responses::dm::dm_message::DmMessageDto,
};

#[derive(Debug, thiserror::Error)]
pub enum SearchDmMessagesError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Not friends with this user")]
    NotFriends,
    #[error("Search query cannot be empty")]
    EmptyQuery,
}

pub struct SearchDmMessages<M, F>
where
    M: DmMessageRepository,
    F: FriendshipRepository,
{
    dm_repository: Arc<M>,
    friendship_repository: Arc<F>,
}

impl<M, F> SearchDmMessages<M, F>
where
    M: DmMessageRepository,
    F: FriendshipRepository,
{
    pub fn new(dm_repository: Arc<M>, friendship_repository: Arc<F>) -> Self {
        Self {
            dm_repository,
            friendship_repository,
        }
    }

    pub async fn execute(
        &self,
        request: SearchDmMessagesRequest,
        requester_id: UserId,
    ) -> Result<PaginatedResult<DmMessageDto>, SearchDmMessagesError> {
        let trimmed = request.query.trim().to_string();
        if trimmed.is_empty() {
            return Err(SearchDmMessagesError::EmptyQuery);
        }

        let friend_id = UserId::new(request.friend_id);

        let pair = self
            .friendship_repository
            .find_pair(&requester_id, &friend_id)
            .await?;
        if !matches!(pair, Some(ref f) if f.status == FriendshipStatus::Accepted) {
            return Err(SearchDmMessagesError::NotFriends);
        }

        let result = self
            .dm_repository
            .search_between(&requester_id, &friend_id, &trimmed, request.pagination)
            .await?;

        Ok(PaginatedResult::new(
            result.items.into_iter().map(DmMessageDto::from).collect(),
            result.total,
            result.pagination,
        ))
    }
}
