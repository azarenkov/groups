use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use ports::repository::RepositoryError;

#[async_trait]
pub trait BannerRepository: Send + Sync {
    async fn get_banner_key(&self, user_id: &UserId) -> Result<Option<String>, RepositoryError>;
    async fn set_banner_key(
        &self,
        user_id: &UserId,
        key: Option<&str>,
    ) -> Result<Option<String>, RepositoryError>;
    async fn apply_tier(
        &self,
        user_id: &UserId,
        tier: &str,
        valid_until: Option<DateTime<Utc>>,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError>;
    async fn is_paid(&self, user_id: &UserId, now: DateTime<Utc>) -> Result<bool, RepositoryError>;
}
