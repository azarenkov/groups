use async_trait::async_trait;
use domain::{
    media::repository::MediaRepository, server::id::ServerId,
    shared::repository::error::RepositoryError, user::id::UserId,
};
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
            Some(key) => sqlx::query_scalar!(
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
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error),
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
            Some(key) => sqlx::query_scalar!(
                r#"
                WITH old AS (
                    SELECT server_id, storage_key FROM media_server_logos
                    WHERE server_id = $1 FOR UPDATE
                ), ins AS (
                    INSERT INTO media_server_logos (server_id, storage_key)
                    VALUES ($1, $2)
                    ON CONFLICT (server_id) DO UPDATE SET storage_key = EXCLUDED.storage_key
                ), q AS (
                    INSERT INTO storage_cleanup_queue (storage_key)
                    SELECT storage_key FROM old
                    WHERE storage_key IS DISTINCT FROM $2
                    ON CONFLICT (storage_key) DO NOTHING
                )
                SELECT storage_key FROM old
                "#,
                server_id.uuid(),
                key,
            )
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error),
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
            r#"SELECT owner_id FROM media_server_owners WHERE server_id = $1"#,
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
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            INSERT INTO media_server_owners (server_id, owner_id)
            VALUES ($1, $2)
            ON CONFLICT (server_id) DO UPDATE SET owner_id = EXCLUDED.owner_id
            "#,
            server_id.uuid(),
            owner_id.uuid(),
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn remove_server_media(&self, server_id: &ServerId) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
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

        sqlx::query!(
            r#"DELETE FROM media_server_owners WHERE server_id = $1"#,
            server_id.uuid(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }
}
