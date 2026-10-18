use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::{plan::Tier, user::id::UserId};
use infrastructure_common::database::postgres::error::map_sqlx_error_to_domain_error;
use ports::{plan_directory::PlanDirectory, repository::RepositoryError};
use sqlx::{Pool, Postgres};

pub struct PostgresPlanDirectory {
    pool: Pool<Postgres>,
    caught_up: AtomicBool,
}

impl PostgresPlanDirectory {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool, caught_up: AtomicBool::new(false) }
    }
}

#[async_trait]
impl PlanDirectory for PostgresPlanDirectory {
    async fn apply_tier(
        &self,
        user_id: &UserId,
        tier: Tier,
        valid_until: Option<DateTime<Utc>>,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            INSERT INTO media_user_tiers (user_id, tier, valid_until, updated_at)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id) DO UPDATE
                SET tier = EXCLUDED.tier,
                    valid_until = EXCLUDED.valid_until,
                    updated_at = EXCLUDED.updated_at
                WHERE media_user_tiers.updated_at <= EXCLUDED.updated_at
            "#,
            user_id.uuid(),
            tier.as_str(),
            valid_until,
            occurred_at,
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn tier_of(&self, user_id: &UserId, now: DateTime<Utc>) -> Result<Tier, RepositoryError> {
        let tier = sqlx::query_scalar!(
            r#"SELECT tier FROM media_user_tiers WHERE user_id = $1 AND (valid_until IS NULL OR valid_until > $2)"#,
            user_id.uuid(),
            now,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(tier.map_or(Tier::Free, |t| Tier::parse(&t)))
    }

    fn is_caught_up(&self) -> bool {
        self.caught_up.load(Ordering::SeqCst)
    }

    fn set_caught_up(&self, caught_up: bool) {
        self.caught_up.store(caught_up, Ordering::SeqCst);
    }
}
