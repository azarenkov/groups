use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::{plan::Tier, user::id::UserId};
use ports::{plan_directory::PlanDirectory, repository::RepositoryError};
use sqlx::{Pool, Postgres, Row};

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

pub struct PostgresPlanDirectory {
    pool: Pool<Postgres>,
    table: &'static str,
    caught_up: AtomicBool,
}

impl PostgresPlanDirectory {
    pub fn voice(pool: Pool<Postgres>) -> Self {
        Self::new(pool, "voice_user_tiers")
    }

    pub fn messaging(pool: Pool<Postgres>) -> Self {
        Self::new(pool, "messaging_user_tiers")
    }

    pub fn media(pool: Pool<Postgres>) -> Self {
        Self::new(pool, "media_user_tiers")
    }

    fn new(pool: Pool<Postgres>, table: &'static str) -> Self {
        Self {
            pool,
            table,
            caught_up: AtomicBool::new(false),
        }
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
        sqlx::query(&format!(
            r#"
            INSERT INTO {table} (user_id, tier, valid_until, updated_at)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id) DO UPDATE
                SET tier = EXCLUDED.tier,
                    valid_until = EXCLUDED.valid_until,
                    updated_at = EXCLUDED.updated_at
                WHERE {table}.updated_at <= EXCLUDED.updated_at
            "#,
            table = self.table
        ))
        .bind(user_id.uuid())
        .bind(tier.as_str())
        .bind(valid_until)
        .bind(occurred_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn tier_of(&self, user_id: &UserId, now: DateTime<Utc>) -> Result<Tier, RepositoryError> {
        let row = sqlx::query(&format!(
            "SELECT tier FROM {} WHERE user_id = $1 AND (valid_until IS NULL OR valid_until > $2)",
            self.table
        ))
        .bind(user_id.uuid())
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map_or(Tier::Free, |r| Tier::parse(&r.get::<String, _>(0))))
    }

    fn is_caught_up(&self) -> bool {
        self.caught_up.load(Ordering::SeqCst)
    }

    fn set_caught_up(&self, caught_up: bool) {
        self.caught_up.store(caught_up, Ordering::SeqCst);
    }
}
