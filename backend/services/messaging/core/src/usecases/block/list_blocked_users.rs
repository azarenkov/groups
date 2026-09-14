use std::sync::Arc;

use domain::user::id::UserId;
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

    pub async fn execute(&self, blocker_id: UserId) -> Result<Vec<BlockedUserView>, RepositoryError> {
        self.friendship_repository.list_blocked(&blocker_id).await
    }
}
