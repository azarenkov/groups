use std::sync::Arc;

use chrono::{DateTime, Utc};
use messaging_domain::{friendship::repository::FriendshipRepository, friendship::status::FriendshipStatus, read_state::repository::ReadStateRepository};
use domain::user::id::UserId;
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum GetDmReadReceiptError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Not friends")]
    NotFriends,
}

pub struct GetDmReadReceipt<R: ReadStateRepository, F: FriendshipRepository> {
    repo: Arc<R>,
    friendship_repo: Arc<F>,
}

impl<R: ReadStateRepository, F: FriendshipRepository> GetDmReadReceipt<R, F> {
    pub fn new(repo: Arc<R>, friendship_repo: Arc<F>) -> Self {
        Self {
            repo,
            friendship_repo,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        friend_id: UserId,
    ) -> Result<Option<DateTime<Utc>>, GetDmReadReceiptError> {
        let pair = self
            .friendship_repo
            .find_pair(&user_id, &friend_id)
            .await?;
        let accepted = matches!(
            pair.as_ref().map(|f| &f.status),
            Some(&FriendshipStatus::Accepted),
        );
        if !accepted {
            return Err(GetDmReadReceiptError::NotFriends);
        }
        let read_at = self.repo.dm_peer_read_at(&user_id, &friend_id).await?;
        Ok(read_at)
    }
}
