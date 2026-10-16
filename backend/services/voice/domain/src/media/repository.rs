use async_trait::async_trait;
use chrono::{DateTime, Utc};

use domain::user::id::UserId;
use ports::repository::RepositoryError;

use super::MediaState;

#[async_trait]
pub trait MediaStateRepository: Send + Sync {
    async fn find(&self, user_id: &UserId) -> Result<Option<MediaState>, RepositoryError>;

    async fn save(&self, user_id: &UserId, state: MediaState, now: DateTime<Utc>) -> Result<(), RepositoryError>;
}
