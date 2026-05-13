use async_trait::async_trait;
use domain::{
    shared::{outbox::outbox_record::OutboxRecord, repository::error::RepositoryError},
    user::id::UserId,
    verification::{
        code::VerificationCode, code_hash::CodeHash, id::VerificationCodeId,
        repository::VerificationCodeRepository,
    },
};
use sqlx::{Pool, Postgres};

use crate::database::postgres::{
    dto::{
        verification_channel::VerificationChannelDto, verification_purpose::VerificationPurposeDto,
    },
    error::map_sqlx_error_to_domain_error,
    repositories::outbox_record::save_outbox_records,
};

pub struct PostgresVerificationCodeRepository {
    pool: Pool<Postgres>,
}

impl PostgresVerificationCodeRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl VerificationCodeRepository for PostgresVerificationCodeRepository {
    async fn save_verification_code(
        &self,
        verification_code: &VerificationCode,
    ) -> Result<(), RepositoryError> {
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
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(())
    }

    async fn save_verification_code_with_events(
        &self,
        verification_code: &VerificationCode,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

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
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        save_outbox_records(&mut tx, events).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;

        Ok(())
    }

    async fn find_verification_code_by_user_id(
        &self,
        user_id: &UserId,
    ) -> Result<VerificationCode, RepositoryError> {
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
            "#,
            user_id.uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?
        .ok_or(RepositoryError::NotFound)?;

        Ok(VerificationCode::restore(
            VerificationCodeId::new(row.id),
            UserId::new(row.user_id),
            row.channel.into(),
            CodeHash::new(row.code_hash),
            row.purpose.into(),
            row.expires_at,
            row.consumed_at,
            row.attempts as u8,
            row.created_at,
        ))
    }

    async fn increment_attempts(
        &self,
        id: &VerificationCodeId,
        max_attempts: u8,
    ) -> Result<Option<u8>, RepositoryError> {
        let row = sqlx::query!(
            r#"
                UPDATE verification_codes
                SET attempts = attempts + 1
                WHERE id = $1 AND attempts < $2
                RETURNING attempts
            "#,
            id.uuid(),
            i16::from(max_attempts),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(row.map(|r| r.attempts as u8))
    }
}
