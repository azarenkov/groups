use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredActivity {
    pub game_id: String,
    pub name: String,
    pub started_at: DateTime<Utc>,
    pub published: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ActivityStoreError {
    #[error("Activity store unavailable: {0}")]
    Unavailable(String),
}

#[async_trait]
pub trait ActivityStore: Send + Sync {
    async fn get(&self, user_id: &UserId) -> Result<Option<StoredActivity>, ActivityStoreError>;
    async fn set(
        &self,
        user_id: &UserId,
        activity: &StoredActivity,
        ttl: Duration,
    ) -> Result<Option<StoredActivity>, ActivityStoreError>;
    async fn clear(&self, user_id: &UserId) -> Result<Option<StoredActivity>, ActivityStoreError>;
    async fn get_many(
        &self,
        user_ids: &[UserId],
    ) -> Result<Vec<(UserId, StoredActivity)>, ActivityStoreError>;
}
