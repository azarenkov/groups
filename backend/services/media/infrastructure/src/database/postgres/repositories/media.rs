use async_trait::async_trait;
use chrono::{DateTime, Utc};
use media_domain::media::repository::MediaRepository;
use domain::{server::id::ServerId, user::id::UserId};
use ports::repository::RepositoryError;
use sqlx::{Pool, Postgres};

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

pub struct PostgresMediaRepository {
    pool: Pool<Postgres>,
}

impl PostgresMediaRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MediaRepository for PostgresMediaRepository {
    async fn get_user_avatar_key(
        &self,
        user_id: &UserId,
    ) -> Result<Option<String>, RepositoryError> {
        sqlx::query_scalar!(
            r#"SELECT storage_key FROM media_user_avatars WHERE user_id = $1"#,
            user_id.uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)
    }

    async fn set_user_avatar_key(
        &self,
        user_id: &UserId,
        key: Option<&str>,
    ) -> Result<Option<String>, RepositoryError> {
        match key {
            Some(key) => {
                let mut tx = self.pool.begin().await.map_err(map_sqlx_error_to_domain_error)?;
                sqlx::query!(
                    "SELECT pg_advisory_xact_lock(hashtextextended($1, 44))",
                    user_id.uuid().to_string()
                )
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
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
                let old = sqlx::query_scalar!(
                    r#"
                    WITH old AS (
                        SELECT user_id, storage_key FROM media_user_avatars
                        WHERE user_id = $1 FOR UPDATE
                    ), ins AS (
                        INSERT INTO media_user_avatars (user_id, storage_key)
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
                .map_err(map_sqlx_error_to_domain_error)?;
                tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
                Ok(old)
            }
            None => sqlx::query_scalar!(
                r#"
                WITH old AS (
                    DELETE FROM media_user_avatars WHERE user_id = $1
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
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error),
        }
    }

    async fn mark_user_deleted(
        &self,
        user_id: &UserId,
        deleted_at: DateTime<Utc>,
    ) -> Result<Option<String>, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error_to_domain_error)?;
        sqlx::query!(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 44))",
            user_id.uuid().to_string()
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        sqlx::query!(
            r#"
            INSERT INTO media_deleted_users (user_id, deleted_at)
            VALUES ($1, $2)
            ON CONFLICT (user_id) DO NOTHING
            "#,
            user_id.uuid(),
            deleted_at,
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let old = sqlx::query_scalar!(
            r#"
            WITH old AS (
                DELETE FROM media_user_avatars WHERE user_id = $1
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
        .map_err(map_sqlx_error_to_domain_error)?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(old)
    }

    async fn get_server_logo_key(
        &self,
        server_id: &ServerId,
    ) -> Result<Option<String>, RepositoryError> {
        sqlx::query_scalar!(
            r#"SELECT storage_key FROM media_server_logos WHERE server_id = $1"#,
            server_id.uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)
    }

    async fn set_server_logo_key(
        &self,
        server_id: &ServerId,
        key: Option<&str>,
    ) -> Result<Option<String>, RepositoryError> {
        match key {
            Some(key) => {
                let row = sqlx::query!(
                    r#"
                    WITH live AS (
                        SELECT server_id FROM media_server_owners
                        WHERE server_id = $1 AND deleted_at IS NULL
                        FOR UPDATE
                    ), old AS (
                        SELECT server_id, storage_key FROM media_server_logos
                        WHERE server_id = $1 AND EXISTS (SELECT 1 FROM live)
                        FOR UPDATE
                    ), ins AS (
                        INSERT INTO media_server_logos (server_id, storage_key)
                        SELECT server_id, $2 FROM live
                        ON CONFLICT (server_id) DO UPDATE SET storage_key = EXCLUDED.storage_key
                    ), q AS (
                        INSERT INTO storage_cleanup_queue (storage_key)
                        SELECT storage_key FROM old
                        WHERE storage_key IS DISTINCT FROM $2
                        ON CONFLICT (storage_key) DO NOTHING
                    )
                    SELECT
                        EXISTS(SELECT 1 FROM live) AS "live!",
                        (SELECT storage_key FROM old) AS old_key
                    "#,
                    server_id.uuid(),
                    key,
                )
                .fetch_one(&self.pool)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
                if !row.live {
                    return Err(RepositoryError::NotFound);
                }
                Ok(row.old_key)
            }
            None => sqlx::query_scalar!(
                r#"
                WITH old AS (
                    DELETE FROM media_server_logos WHERE server_id = $1
                    RETURNING storage_key
                ), q AS (
                    INSERT INTO storage_cleanup_queue (storage_key)
                    SELECT storage_key FROM old
                    ON CONFLICT (storage_key) DO NOTHING
                )
                SELECT storage_key FROM old
                "#,
                server_id.uuid(),
            )
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error),
        }
    }

    async fn get_server_owner(
        &self,
        server_id: &ServerId,
    ) -> Result<Option<UserId>, RepositoryError> {
        let owner = sqlx::query_scalar!(
            r#"SELECT owner_id FROM media_server_owners WHERE server_id = $1 AND deleted_at IS NULL"#,
            server_id.uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(owner.map(UserId::new))
    }

    async fn upsert_server_owner(
        &self,
        server_id: &ServerId,
        owner_id: &UserId,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            INSERT INTO media_server_owners (server_id, owner_id, updated_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (server_id) DO UPDATE
                SET owner_id = EXCLUDED.owner_id, updated_at = EXCLUDED.updated_at
                WHERE media_server_owners.deleted_at IS NULL
                  AND media_server_owners.updated_at <= EXCLUDED.updated_at
            "#,
            server_id.uuid(),
            owner_id.uuid(),
            occurred_at,
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn remove_server_media(
        &self,
        server_id: &ServerId,
        owner_id: &UserId,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        sqlx::query!(
            r#"
            INSERT INTO media_server_owners (server_id, owner_id, updated_at, deleted_at)
            VALUES ($1, $2, $3, $3)
            ON CONFLICT (server_id) DO UPDATE
                SET deleted_at = EXCLUDED.deleted_at,
                    updated_at = GREATEST(media_server_owners.updated_at, EXCLUDED.updated_at)
            "#,
            server_id.uuid(),
            owner_id.uuid(),
            occurred_at,
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        sqlx::query!(
            r#"
            WITH old AS (
                DELETE FROM media_server_logos WHERE server_id = $1
                RETURNING storage_key
            )
            INSERT INTO storage_cleanup_queue (storage_key)
            SELECT storage_key FROM old
            ON CONFLICT (storage_key) DO NOTHING
            "#,
            server_id.uuid(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }
}
