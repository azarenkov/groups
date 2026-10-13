use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::{plan::Tier, user::id::UserId};
use ports::{plan_directory::PlanDirectory, repository::RepositoryError};
use sqlx::{Pool, Postgres, Row};

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

pub struct PostgresPlanDirectory {
    pool: Pool<Postgres>,
    table: &'static str,
}

impl PostgresPlanDirectory {
    pub fn voice(pool: Pool<Postgres>) -> Self {
        Self {
            pool,
            table: "voice_user_profiles",
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
            INSERT INTO {table} (user_id, name, updated_at, tier, tier_valid_until, tier_updated_at)
            VALUES ($1, '', to_timestamp(0), $2, $3, $4)
            ON CONFLICT (user_id) DO UPDATE
                SET tier = EXCLUDED.tier,
                    tier_valid_until = EXCLUDED.tier_valid_until,
                    tier_updated_at = EXCLUDED.tier_updated_at
                WHERE {table}.tier_updated_at IS NULL
                   OR {table}.tier_updated_at <= EXCLUDED.tier_updated_at
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
            "SELECT tier FROM {} WHERE user_id = $1 AND (tier_valid_until IS NULL OR tier_valid_until > $2)",
            self.table
        ))
        .bind(user_id.uuid())
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map_or(Tier::Free, |r| Tier::parse(&r.get::<String, _>(0))))
    }
}
