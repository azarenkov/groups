use async_trait::async_trait;
use domain::{user::id::UserId, user::name::UserName};
use ports::repository::RepositoryError;
use identity_domain::{events::IdentityEvent, refresh_token::RefreshToken, unit_of_work::IdentityTransaction, unit_of_work::IdentityUnitOfWork, user::User, verification::code::VerificationCode};
use sqlx::{Pool, Postgres, Transaction};

use crate::database::postgres::{
    dto::{
        verification_channel::VerificationChannelDto, verification_purpose::VerificationPurposeDto,
    },
    error::map_sqlx_error_to_domain_error,
    repositories::outbox_record::save_outbox_records,
};
use crate::outbox::mapping::to_record;

pub struct PostgresIdentityUnitOfWork {
    pool: Pool<Postgres>,
}

impl PostgresIdentityUnitOfWork {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

pub struct PostgresIdentityTransaction {
    tx: Transaction<'static, Postgres>,
}

#[async_trait]
impl IdentityUnitOfWork for PostgresIdentityUnitOfWork {
    async fn begin(&self) -> Result<Box<dyn IdentityTransaction>, RepositoryError> {
        let tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        Ok(Box::new(PostgresIdentityTransaction { tx }))
    }
}

#[async_trait]
impl IdentityTransaction for PostgresIdentityTransaction {
    async fn insert_user(&mut self, user: &User) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
                INSERT INTO users (
                    id, name, email, password_hash, created_at, updated_at,
                    locale, email_verified_at, phone_verified_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            user.id().uuid(),
            user.name().as_str(),
            user.email().as_str(),
            user.password_hash().as_str(),
            user.created_at(),
            user.updated_at(),
            user.locale().as_str(),
            user.email_verified_at(),
            user.phone_verified_at(),
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn insert_refresh_token(&mut self, refresh_token: &RefreshToken) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
                INSERT INTO refresh_tokens (
                    id, user_id, expires_at, revoked_at, created_at,
                    last_used_at, last_ip, last_user_agent
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            refresh_token.id().uuid(),
            refresh_token.user_id().uuid(),
            refresh_token.expires_at(),
            refresh_token.revoked_at(),
            refresh_token.created_at(),
            refresh_token.last_used_at(),
            refresh_token.last_ip(),
            refresh_token.last_user_agent(),
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn revoke_refresh_token(&mut self, refresh_token: &RefreshToken) -> Result<bool, RepositoryError> {
        let result = sqlx::query!(
            r#"
                UPDATE refresh_tokens
                SET revoked_at = $2
                WHERE id = $1 AND revoked_at IS NULL
            "#,
            refresh_token.id().uuid(),
            refresh_token.revoked_at(),
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(result.rows_affected() > 0)
    }

    async fn set_user_name(&mut self, user_id: &UserId, name: &UserName) -> Result<(), RepositoryError> {
        let result = sqlx::query!(
            r#"
                UPDATE users
                SET name = $2, updated_at = now()
                WHERE id = $1
            "#,
            user_id.uuid(),
            name.as_str(),
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        Ok(())
    }

    async fn insert_verification_code(&mut self, verification_code: &VerificationCode) -> Result<(), RepositoryError> {
        let channel = VerificationChannelDto::from(verification_code.channel());
        let purpose = VerificationPurposeDto::from(verification_code.purpose());
        sqlx::query!(
            r#"
                INSERT INTO verification_codes (
                    id, user_id, channel, code_hash, purpose,
                    expires_at, consumed_at, attempts, created_at
                )
                VALUES (
                    $1, $2, $3, $4, $5,
                    $6, $7, $8, $9
                )
            "#,
            verification_code.id().uuid(),
            verification_code.user_id().uuid(),
            channel as VerificationChannelDto,
            verification_code.code_hash().as_str(),
            purpose as VerificationPurposeDto,
            verification_code.expires_at(),
            verification_code.consumed_at(),
            i16::from(verification_code.attempts()),
            verification_code.created_at(),
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn mark_user_verified(&mut self, user: &User) -> Result<(), RepositoryError> {
        let result = sqlx::query!(
            r#"
                UPDATE users
                SET email_verified_at = $2,
                    phone_verified_at = $3
                WHERE id = $1
            "#,
            user.id().uuid(),
            user.email_verified_at(),
            user.phone_verified_at(),
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        Ok(())
    }

    async fn consume_verification_code(&mut self, verification_code: &VerificationCode) -> Result<bool, RepositoryError> {
        let result = sqlx::query!(
            r#"
                UPDATE verification_codes
                SET consumed_at = $2
                WHERE id = $1 AND consumed_at IS NULL
            "#,
            verification_code.id().uuid(),
            verification_code.consumed_at(),
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(result.rows_affected() > 0)
    }

    async fn save_events(&mut self, events: &[IdentityEvent], request_id: Option<&str>) -> Result<(), RepositoryError> {
        let records = events
            .iter()
            .map(|e| to_record(e, request_id))
            .collect::<Result<Vec<_>, _>>()?;
        save_outbox_records(&mut self.tx, &records).await
    }

    async fn commit(self: Box<Self>) -> Result<(), RepositoryError> {
        self.tx.commit().await.map_err(map_sqlx_error_to_domain_error)
    }
}
