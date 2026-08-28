use async_trait::async_trait;
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use chrono::{DateTime, Utc};
use identity_domain::{events::IdentityEvent, refresh_token::RefreshToken, refresh_token::id::RefreshTokenId, unit_of_work::IdentityTransaction, unit_of_work::IdentityUnitOfWork, user::User, verification::code::VerificationCode, verification::code_hash::CodeHash, verification::id::VerificationCodeId, verification::purpose::VerificationPurpose};
use sqlx::{Pool, Postgres, Transaction};

use crate::database::postgres::{
    dto::{
        user::UserDto, verification_channel::VerificationChannelDto,
        verification_purpose::VerificationPurposeDto,
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

    async fn load_user_for_update(&mut self, user_id: &UserId) -> Result<Option<User>, RepositoryError> {
        let dto = sqlx::query_as!(
            UserDto,
            r#"
            SELECT id, name, email, password_hash, created_at, updated_at,
                   email_verified_at, phone_verified_at, locale, pending_email, password_changed_at, disabled_at, deleted_at
            FROM users
            WHERE id = $1
            FOR UPDATE
            "#,
            user_id.uuid(),
        )
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(dto.map(User::from))
    }

    async fn load_pending_verification_for_update(&mut self, user_id: &UserId) -> Result<Option<VerificationCode>, RepositoryError> {
        let row = sqlx::query!(
            r#"
                SELECT
                    id,
                    user_id,
                    channel AS "channel: VerificationChannelDto",
                    code_hash,
                    purpose AS "purpose: VerificationPurposeDto",
                    expires_at,
                    consumed_at,
                    attempts,
                    created_at
                FROM verification_codes
                WHERE user_id = $1 AND consumed_at IS NULL
                ORDER BY created_at DESC
                LIMIT 1
                FOR UPDATE
            "#,
            user_id.uuid(),
        )
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map(|row| {
            VerificationCode::restore(
                VerificationCodeId::new(row.id),
                UserId::new(row.user_id),
                row.channel.into(),
                CodeHash::new(row.code_hash),
                row.purpose.into(),
                row.expires_at,
                row.consumed_at,
                row.attempts as u8,
                row.created_at,
            )
        }))
    }

    async fn load_pending_verification_for_purpose_for_update(&mut self, user_id: &UserId, purpose: VerificationPurpose) -> Result<Option<VerificationCode>, RepositoryError> {
        let purpose_dto = VerificationPurposeDto::from(&purpose);
        let row = sqlx::query!(
            r#"
                SELECT
                    id,
                    user_id,
                    channel AS "channel: VerificationChannelDto",
                    code_hash,
                    purpose AS "purpose: VerificationPurposeDto",
                    expires_at,
                    consumed_at,
                    attempts,
                    created_at
                FROM verification_codes
                WHERE user_id = $1 AND consumed_at IS NULL AND purpose = $2
                ORDER BY created_at DESC
                LIMIT 1
                FOR UPDATE
            "#,
            user_id.uuid(),
            purpose_dto as VerificationPurposeDto,
        )
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map(|row| {
            VerificationCode::restore(
                VerificationCodeId::new(row.id),
                UserId::new(row.user_id),
                row.channel.into(),
                CodeHash::new(row.code_hash),
                row.purpose.into(),
                row.expires_at,
                row.consumed_at,
                row.attempts as u8,
                row.created_at,
            )
        }))
    }

    async fn delete_oauth_identities(&mut self, user_id: &UserId) -> Result<(), RepositoryError> {
        sqlx::query!("DELETE FROM oauth_identities WHERE user_id = $1", user_id.uuid())
            .execute(&mut *self.tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn revoke_refresh_tokens_for_user(&mut self, user_id: &UserId, keep: Option<&RefreshTokenId>, now: DateTime<Utc>) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
                UPDATE refresh_tokens
                SET revoked_at = $2
                WHERE user_id = $1
                  AND revoked_at IS NULL
                  AND ($3::uuid IS NULL OR id <> $3)
            "#,
            user_id.uuid(),
            now,
            keep.map(|k| k.uuid()),
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn load_refresh_token_for_update(&mut self, id: &RefreshTokenId) -> Result<Option<RefreshToken>, RepositoryError> {
        let row = sqlx::query!(
            r#"
                SELECT id, user_id, expires_at, revoked_at, created_at,
                       last_used_at, last_ip, last_user_agent
                FROM refresh_tokens
                WHERE id = $1
                FOR UPDATE
            "#,
            id.uuid(),
        )
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map(|row| {
            RefreshToken::restore(
                RefreshTokenId::new(row.id),
                UserId::new(row.user_id),
                row.expires_at,
                row.revoked_at,
                row.created_at,
                row.last_used_at,
                row.last_ip,
                row.last_user_agent,
            )
        }))
    }

    async fn save_user(&mut self, user: &User) -> Result<(), RepositoryError> {
        let result = sqlx::query!(
            r#"
                UPDATE users
                SET name = $2,
                    locale = $3,
                    email_verified_at = $4,
                    phone_verified_at = $5,
                    updated_at = $6,
                    email = $7,
                    password_hash = $8,
                    pending_email = $9,
                    password_changed_at = $10,
                    disabled_at = $11,
                    deleted_at = $12
                WHERE id = $1
            "#,
            user.id().uuid(),
            user.name().as_str(),
            user.locale().as_str(),
            user.email_verified_at(),
            user.phone_verified_at(),
            user.updated_at(),
            user.email().as_str(),
            user.password_hash().as_str(),
            user.pending_email().map(|e| e.as_str()),
            user.password_changed_at(),
            user.disabled_at(),
            user.deleted_at(),
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        Ok(())
    }

    async fn save_refresh_token(&mut self, refresh_token: &RefreshToken) -> Result<(), RepositoryError> {
        let result = sqlx::query!(
            r#"
                UPDATE refresh_tokens
                SET revoked_at = $2, last_used_at = $3
                WHERE id = $1
            "#,
            refresh_token.id().uuid(),
            refresh_token.revoked_at(),
            refresh_token.last_used_at(),
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        Ok(())
    }

    async fn save_verification_code(&mut self, verification_code: &VerificationCode) -> Result<(), RepositoryError> {
        let result = sqlx::query!(
            r#"
                UPDATE verification_codes
                SET attempts = $2, consumed_at = $3
                WHERE id = $1
            "#,
            verification_code.id().uuid(),
            i16::from(verification_code.attempts()),
            verification_code.consumed_at(),
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        Ok(())
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
