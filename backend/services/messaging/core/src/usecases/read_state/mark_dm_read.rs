use std::sync::Arc;

use messaging_domain::{dm_message::repository::DmMessageRepository, friendship::repository::FriendshipRepository, friendship::status::FriendshipStatus, read_state::repository::ReadStateRepository};
use domain::user::id::UserId;
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum MarkDmReadError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("No conversation with this user")]
    NoConversation,
}

pub struct MarkDmRead<R: ReadStateRepository, F: FriendshipRepository, M: DmMessageRepository> {
    repo: Arc<R>,
    friendship_repo: Arc<F>,
    dm_repo: Arc<M>,
}

impl<R: ReadStateRepository, F: FriendshipRepository, M: DmMessageRepository> MarkDmRead<R, F, M> {
    pub fn new(repo: Arc<R>, friendship_repo: Arc<F>, dm_repo: Arc<M>) -> Self {
        Self {
            repo,
            friendship_repo,
            dm_repo,
        }
    }

    async fn ensure_conversation(
        &self,
        user_id: &UserId,
        friend_id: &UserId,
    ) -> Result<(), MarkDmReadError> {
        let pair = self.friendship_repo.find_pair(user_id, friend_id).await?;
        if matches!(pair.as_ref().map(|f| &f.status), Some(&FriendshipStatus::Accepted)) {
            return Ok(());
        }
        if self.dm_repo.has_conversation(user_id, friend_id).await? {
            return Ok(());
        }
        Err(MarkDmReadError::NoConversation)
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        friend_id: UserId,
    ) -> Result<(), MarkDmReadError> {
        self.ensure_conversation(&user_id, &friend_id).await?;
        self.repo.mark_dm_read(&user_id, &friend_id).await?;
        Ok(())
    }
}
