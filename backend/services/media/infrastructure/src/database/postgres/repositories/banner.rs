use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use media_domain::banner::BannerRepository;
use ports::repository::RepositoryError;
use sqlx::{Pool, Postgres};

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

pub struct PostgresBannerRepository {
    pool: Pool<Postgres>,
}

impl PostgresBannerRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BannerRepository for PostgresBannerRepository {
    async fn get_banner_key(&self, user_id: &UserId) -> Result<Option<String>, RepositoryError> {
        sqlx::query_scalar!(
            r#"SELECT storage_key FROM media_user_banners WHERE user_id = $1"#,
            user_id.uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)
    }

    async fn set_banner_key(
        &self,
        user_id: &UserId,
        key: Option<&str>,
    ) -> Result<Option<String>, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error_to_domain_error)?;
        sqlx::query!(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 44))",
            user_id.uuid().to_string()
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let old = match key {
            Some(key) => {
                let deleted = sqlx::query_scalar!(
                    r#"SELECT EXISTS(SELECT 1 FROM media_deleted_users WHERE user_id = $1) AS "exists!""#,
                    user_id.uuid()
                )
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
                if deleted {
                    return Err(RepositoryError::NotFound);
                }
                sqlx::query_scalar!(
                    r#"
                    WITH old AS (
                        SELECT user_id, storage_key FROM media_user_banners
                        WHERE user_id = $1 FOR UPDATE
                    ), ins AS (
                        INSERT INTO media_user_banners (user_id, storage_key)
                        VALUES ($1, $2)
                        ON CONFLICT (user_id) DO UPDATE SET storage_key = EXCLUDED.storage_key
                    ), q AS (
                        INSERT INTO storage_cleanup_queue (storage_key)
                        SELECT storage_key FROM old
                        WHERE storage_key IS DISTINCT FROM $2
                        ON CONFLICT (storage_key) DO NOTHING
                    )
                    SELECT storage_key FROM old
                    "#,
                    user_id.uuid(),
                    key,
                )
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?
            }
            None => sqlx::query_scalar!(
                r#"
                WITH old AS (
                    DELETE FROM media_user_banners WHERE user_id = $1
                    RETURNING storage_key
                ), q AS (
                    INSERT INTO storage_cleanup_queue (storage_key)
                    SELECT storage_key FROM old
                    ON CONFLICT (storage_key) DO NOTHING
                )
                SELECT storage_key FROM old
                "#,
                user_id.uuid(),
            )
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?,
        };
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(old)
    }

    async fn apply_tier(
        &self,
        user_id: &UserId,
        tier: &str,
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
            tier,
            valid_until,
            occurred_at,
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn is_paid(&self, user_id: &UserId, now: DateTime<Utc>) -> Result<bool, RepositoryError> {
        sqlx::query_scalar!(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM media_user_tiers
                WHERE user_id = $1
                  AND tier <> 'free'
                  AND (valid_until IS NULL OR valid_until > $2)
            ) AS "exists!"
            "#,
            user_id.uuid(),
            now,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)
    }
}
