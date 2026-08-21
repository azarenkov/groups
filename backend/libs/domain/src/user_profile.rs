use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::{shared::repository::error::RepositoryError, user::id::UserId};

#[async_trait]
pub trait UserProfileRepository {
    async fn get_name(&self, user_id: &UserId) -> Result<Option<String>, RepositoryError>;
    async fn find_user_by_name(&self, name: &str) -> Result<Option<UserId>, RepositoryError>;
    async fn upsert_name(
        &self,
        user_id: &UserId,
        name: &str,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError>;
}
