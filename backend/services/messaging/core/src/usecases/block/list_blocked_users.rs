use std::sync::Arc;

use domain::{pagination::{KeysetCursor, Page}, user::id::UserId};
use messaging_domain::friendship::{block::BlockedUserView, repository::FriendshipRepository};
use ports::repository::RepositoryError;

pub struct ListBlockedUsers<F: FriendshipRepository> {
    friendship_repository: Arc<F>,
}

impl<F: FriendshipRepository> ListBlockedUsers<F> {
    pub fn new(friendship_repository: Arc<F>) -> Self {
        Self {
            friendship_repository,
        }
    }

    pub async fn execute(
        &self,
        blocker_id: UserId,
        cursor: Option<KeysetCursor>,
        limit: i64,
    ) -> Result<Page<BlockedUserView>, RepositoryError> {
        let rows = self
            .friendship_repository
            .list_blocked(&blocker_id, cursor.as_ref(), limit)
            .await?;
        Ok(Page::from_overfetch(rows, limit, |b| {
            KeysetCursor::new(b.created_at, b.user_id.uuid().to_string())
        }))
    }

    pub async fn ids(&self, blocker_id: UserId) -> Result<Vec<UserId>, RepositoryError> {
        self.friendship_repository.list_blocked_ids(&blocker_id).await
    }
}
