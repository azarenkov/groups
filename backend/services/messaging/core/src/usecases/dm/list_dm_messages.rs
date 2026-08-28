use std::sync::Arc;

use messaging_domain::{dm_message::id::DmMessageId, dm_message::repository::DmMessageRepository};
use domain::user::id::UserId;
use ports::repository::RepositoryError;

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
    #[error("Invalid pagination cursor")]
    InvalidCursor,
    #[error("Cannot specify both before and after cursors")]
    ConflictingCursors,
}

pub struct ListDmMessages<M>
where
    M: DmMessageRepository,
{
    dm_repository: Arc<M>,
}

impl<M> ListDmMessages<M>
where
    M: DmMessageRepository,
{
    pub fn new(dm_repository: Arc<M>) -> Self {
        Self { dm_repository }
    }

    pub async fn execute(
        &self,
        request: ListDmMessagesRequest,
        requester_id: UserId,
    ) -> Result<Vec<DmMessageDto>, ListDmMessagesError> {
        let friend_id = UserId::new(request.friend_id);

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
