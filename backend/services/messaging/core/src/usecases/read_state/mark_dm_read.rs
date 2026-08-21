use std::sync::Arc;

use domain::{
    friendship::{repository::FriendshipRepository, status::FriendshipStatus},
    read_state::repository::ReadStateRepository,
    shared::repository::error::RepositoryError,
    user::id::UserId,
};

#[derive(Debug, thiserror::Error)]
pub enum MarkDmReadError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Not friends")]
    NotFriends,
}

pub struct MarkDmRead<R: ReadStateRepository, F: FriendshipRepository> {
    repo: Arc<R>,
    friendship_repo: Arc<F>,
}

impl<R: ReadStateRepository, F: FriendshipRepository> MarkDmRead<R, F> {
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
    ) -> Result<(), MarkDmReadError> {
        let pair = self
            .friendship_repo
            .find_pair(&user_id, &friend_id)
            .await?;
        let accepted = matches!(
            pair.as_ref().map(|f| &f.status),
            Some(&FriendshipStatus::Accepted),
        );
        if !accepted {
            return Err(MarkDmReadError::NotFriends);
        }
        self.repo.mark_dm_read(&user_id, &friend_id).await?;
        Ok(())
    }
}
