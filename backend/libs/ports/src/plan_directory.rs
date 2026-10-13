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

pub type PlanListenerError = Box<dyn std::error::Error + Send + Sync>;

#[async_trait]
pub trait PlanListener: Send + Sync {
    async fn plan_applied(&self, user_id: &UserId, now: DateTime<Utc>, caught_up: bool) -> Result<(), PlanListenerError>;
    async fn caught_up(&self, now: DateTime<Utc>) -> Result<(), PlanListenerError>;
}

pub struct NoPlanListener;

#[async_trait]
impl PlanListener for NoPlanListener {
    async fn plan_applied(&self, _: &UserId, _: DateTime<Utc>, _: bool) -> Result<(), PlanListenerError> {
        Ok(())
    }

    async fn caught_up(&self, _: DateTime<Utc>) -> Result<(), PlanListenerError> {
        Ok(())
    }
}
