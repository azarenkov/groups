use std::sync::Arc;

use messaging_domain::dm_message::repository::DmMessageRepository;
use domain::{pagination::PaginatedResult, user::id::UserId};
use ports::repository::RepositoryError;

use crate::dto::{
    requests::dm::search_dm_messages::SearchDmMessagesRequest,
    responses::dm::dm_message::DmMessageDto,
};

#[derive(Debug, thiserror::Error)]
pub enum SearchDmMessagesError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Search query cannot be empty")]
    EmptyQuery,
}

pub struct SearchDmMessages<M>
where
    M: DmMessageRepository,
{
    dm_repository: Arc<M>,
}

impl<M> SearchDmMessages<M>
where
    M: DmMessageRepository,
{
    pub fn new(dm_repository: Arc<M>) -> Self {
        Self { dm_repository }
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
