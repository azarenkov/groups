use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ports::repository::RepositoryError;

#[async_trait]
pub trait CallLogRetention: Send + Sync {
    async fn forget_consumed_before(&self, before: DateTime<Utc>, limit: i64) -> Result<u64, RepositoryError>;
}
