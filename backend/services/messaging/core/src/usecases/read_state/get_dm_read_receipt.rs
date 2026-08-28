use std::sync::Arc;

use chrono::{DateTime, Utc};
use messaging_domain::read_state::repository::ReadStateRepository;
use domain::user::id::UserId;
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum GetDmReadReceiptError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct GetDmReadReceipt<R: ReadStateRepository> {
    repo: Arc<R>,
}

impl<R: ReadStateRepository> GetDmReadReceipt<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        friend_id: UserId,
    ) -> Result<Option<DateTime<Utc>>, GetDmReadReceiptError> {
        let read_at = self.repo.dm_peer_read_at(&user_id, &friend_id).await?;
        Ok(read_at)
    }
}
