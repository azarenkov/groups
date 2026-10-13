use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::{plan::Tier, user::id::UserId};

use crate::repository::RepositoryError;

#[async_trait]
pub trait PlanDirectory: Send + Sync {
    async fn apply_tier(
        &self,
        user_id: &UserId,
        tier: Tier,
        valid_until: Option<DateTime<Utc>>,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError>;
    async fn tier_of(&self, user_id: &UserId, now: DateTime<Utc>) -> Result<Tier, RepositoryError>;
}
