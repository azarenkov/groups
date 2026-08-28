use std::sync::Arc;

use messaging_domain::read_state::repository::ReadStateRepository;
use domain::user::id::UserId;
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum MarkDmReadError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct MarkDmRead<R: ReadStateRepository> {
    repo: Arc<R>,
}

impl<R: ReadStateRepository> MarkDmRead<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        friend_id: UserId,
    ) -> Result<(), MarkDmReadError> {
        self.repo.mark_dm_read(&user_id, &friend_id).await?;
        Ok(())
    }
}
