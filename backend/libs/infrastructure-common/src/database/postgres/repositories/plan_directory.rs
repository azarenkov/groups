use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::{plan::Tier, user::id::UserId};
use ports::{plan_directory::PlanDirectory, repository::RepositoryError};
use sqlx::{Pool, Postgres, Row};

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

struct PlanTable {
    name: &'static str,
    valid_until: &'static str,
    updated_at: &'static str,
    seed_columns: &'static str,
    seed_values: &'static str,
}

const PROFILE_SEED_COLUMNS: &str = ", name, updated_at";
const PROFILE_SEED_VALUES: &str = ", '', to_timestamp(0)";

pub struct PostgresPlanDirectory {
    pool: Pool<Postgres>,
    table: PlanTable,
}

impl PostgresPlanDirectory {
    pub fn voice(pool: Pool<Postgres>) -> Self {
        Self::profiles(pool, "voice_user_profiles")
    }

    pub fn messaging(pool: Pool<Postgres>) -> Self {
        Self::profiles(pool, "messaging_user_profiles")
    }

    pub fn media(pool: Pool<Postgres>) -> Self {
        Self {
            pool,
            table: PlanTable {
                name: "media_user_tiers",
                valid_until: "valid_until",
                updated_at: "updated_at",
                seed_columns: "",
                seed_values: "",
            },
        }
    }

    fn profiles(pool: Pool<Postgres>, name: &'static str) -> Self {
        Self {
            pool,
            table: PlanTable {
                name,
                valid_until: "tier_valid_until",
                updated_at: "tier_updated_at",
                seed_columns: PROFILE_SEED_COLUMNS,
                seed_values: PROFILE_SEED_VALUES,
            },
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
        let PlanTable { name, valid_until: until, updated_at, seed_columns, seed_values } = &self.table;
        sqlx::query(&format!(
            r#"
            INSERT INTO {name} (user_id, tier, {until}, {updated_at}{seed_columns})
            VALUES ($1, $2, $3, $4{seed_values})
            ON CONFLICT (user_id) DO UPDATE
                SET tier = EXCLUDED.tier,
                    {until} = EXCLUDED.{until},
                    {updated_at} = EXCLUDED.{updated_at}
                WHERE {name}.{updated_at} IS NULL
                   OR {name}.{updated_at} <= EXCLUDED.{updated_at}
            "#
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
        let PlanTable { name, valid_until, .. } = &self.table;
        let row = sqlx::query(&format!(
            "SELECT tier FROM {name} WHERE user_id = $1 AND ({valid_until} IS NULL OR {valid_until} > $2)"
        ))
        .bind(user_id.uuid())
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map_or(Tier::Free, |r| Tier::parse(&r.get::<String, _>(0))))
    }
}
