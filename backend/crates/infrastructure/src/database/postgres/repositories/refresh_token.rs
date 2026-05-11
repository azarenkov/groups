use async_trait::async_trait;
use domain::{
    refresh_token::{
        id::RefreshTokenId, refresh_token::RefreshToken, repository::RefreshTokenRepository,
    },
    shared::repository::error::RepositoryError,
    user::id::UserId,
};
use sqlx::{Pool, Postgres};

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

pub struct PostgresRefreshTokenRepository {
    pool: Pool<Postgres>,
}

impl PostgresRefreshTokenRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RefreshTokenRepository for PostgresRefreshTokenRepository {
    async fn save_refresh_token(
        &self,
        refresh_token: &RefreshToken,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
                INSERT INTO refresh_tokens (
                    id, user_id, expires_at, revoked_at, created_at
                )
                VALUES ($1, $2, $3, $4, $5)
            "#,
            refresh_token.id().uuid(),
            refresh_token.user_id().uuid(),
            refresh_token.expires_at(),
            refresh_token.revoked_at(),
            refresh_token.created_at(),
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(())
    }

    async fn find_by_id(
        &self,
        id: &RefreshTokenId,
    ) -> Result<Option<RefreshToken>, RepositoryError> {
        let row = sqlx::query!(
            r#"
                SELECT
                    id,
                    user_id,
                    expires_at,
                    revoked_at,
                    created_at
                FROM refresh_tokens
                WHERE id = $1
            "#,
            id.uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(row.map(|row| {
            RefreshToken::restore(
                RefreshTokenId::new(row.id),
                UserId::new(row.user_id),
                row.expires_at,
                row.revoked_at,
                row.created_at,
            )
        }))
    }

    async fn revoke(&self, refresh_token: &RefreshToken) -> Result<(), RepositoryError> {
        let result = sqlx::query!(
            r#"
                UPDATE refresh_tokens
                SET revoked_at = $2
                WHERE id = $1
            "#,
            refresh_token.id().uuid(),
            refresh_token.revoked_at(),
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }

        Ok(())
    }

    async fn revoke_all_for_user(&self, user_id: &UserId) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
                UPDATE refresh_tokens
                SET revoked_at = now()
                WHERE user_id = $1 AND revoked_at IS NULL
            "#,
            user_id.uuid(),
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(())
    }
}
