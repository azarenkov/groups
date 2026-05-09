use async_trait::async_trait;
use domain::{
    shared::repository::error::RepositoryError,
    verification::{code::VerificationCode, repository::VerificationCodeRepository},
};
use sqlx::{Pool, Postgres};

use crate::database::postgres::{
    dto::{
        verification_channel::VerificationChannelDto, verification_purpose::VerificationPurposeDto,
    },
    error::map_sqlx_error_to_domain_error,
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
}
