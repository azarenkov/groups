use std::sync::Arc;

use domain::{
    dm_message::{id::DmMessageId, repository::DmMessageRepository},
    friendship::{repository::FriendshipRepository, status::FriendshipStatus},
    shared::repository::error::RepositoryError,
    user::id::UserId,
};

use crate::dto::{
    requests::dm::list_dm_messages::ListDmMessagesRequest,
    responses::dm::dm_message::DmMessageDto,
};

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 100;

#[derive(Debug, thiserror::Error)]
pub enum ListDmMessagesError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Not friends with this user")]
    NotFriends,
    #[error("Invalid pagination cursor")]
    InvalidCursor,
    #[error("Cannot specify both before and after cursors")]
    ConflictingCursors,
}

pub struct ListDmMessages<M, F>
where
    M: DmMessageRepository,
    F: FriendshipRepository,
{
    dm_repository: Arc<M>,
    friendship_repository: Arc<F>,
}

impl<M, F> ListDmMessages<M, F>
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
        request: ListDmMessagesRequest,
        requester_id: UserId,
    ) -> Result<Vec<DmMessageDto>, ListDmMessagesError> {
        let friend_id = UserId::new(request.friend_id);

        let pair = self
            .friendship_repository
            .find_pair(&requester_id, &friend_id)
            .await?;
        if !matches!(pair, Some(ref f) if f.status == FriendshipStatus::Accepted) {
            return Err(ListDmMessagesError::NotFriends);
        }

        let limit = clamp_limit(request.limit);
        let before = request.before.map(DmMessageId::new);
        let after = request.after.map(DmMessageId::new);

        if before.is_some() && after.is_some() {
            return Err(ListDmMessagesError::ConflictingCursors);
        }
        let has_cursor = before.is_some() || after.is_some();

        let rows = self
            .dm_repository
            .list_between(
                &requester_id,
                &friend_id,
                before.as_ref(),
                after.as_ref(),
                limit,
            )
            .await
            .map_err(|e| match e {
                RepositoryError::NotFound if has_cursor => ListDmMessagesError::InvalidCursor,
                other => ListDmMessagesError::Repository(other),
            })?;

        Ok(rows.into_iter().map(DmMessageDto::from).collect())
    }
}

fn clamp_limit(value: i64) -> i64 {
    if value <= 0 {
        DEFAULT_LIMIT
    } else if value > MAX_LIMIT {
        MAX_LIMIT
    } else {
        value
    }
}
