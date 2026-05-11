use async_trait::async_trait;

use domain::{
    shared::{
        outbox::outbox_record::OutboxRecord, repository::error::RepositoryError,
        value_objects::password_hash::PasswordHash,
    },
    user::{User, email::Email, id::UserId, name::UserName, repository::UserRepository},
    verification::code::VerificationCode,
};
use sqlx::{Pool, Postgres};

use crate::database::postgres::{
    dto::{
        user::UserDto, verification_channel::VerificationChannelDto,
        verification_purpose::VerificationPurposeDto,
    },
    error::map_sqlx_error_to_domain_error,
    repositories::outbox_record::save_outbox_records,
};

pub struct PostgresUserRepository {
    pool: Pool<Postgres>,
}

impl PostgresUserRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for PostgresUserRepository {
    async fn save_user(&self, user: &User) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
                INSERT INTO users (id, name, email, password_hash, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            user.id().uuid(),
            user.name().as_str(),
            user.email().as_str(),
            user.password_hash().as_str(),
            user.created_at(),
            user.updated_at()
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(())
    }

    async fn register_user(
        &self,
        user: &User,
        verification_code: &VerificationCode,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        sqlx::query!(
            r#"
                INSERT INTO users (id, name, email, password_hash, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            user.id().uuid(),
            user.name().as_str(),
            user.email().as_str(),
            user.password_hash().as_str(),
            user.created_at(),
            user.updated_at(),
        )
        .execute(&mut *tx)
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

    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, RepositoryError> {
        let dto = sqlx::query_as!(
            UserDto,
            r#"
            SELECT
                id,
                name,
                email,
                password_hash,
                created_at,
                updated_at,
                email_verified_at,
                phone_verified_at
            FROM users
            WHERE id = $1
            "#,
            id.uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(dto.map(User::from))
    }

    async fn get_password_hash_by_email(
        &self,
        email: &Email,
    ) -> Result<PasswordHash, RepositoryError> {
        let record = sqlx::query!(
            r#"
                SELECT password_hash
                FROM users
                WHERE email = $1
            "#,
            email.as_str()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        match record {
            Some(row) => Ok(PasswordHash::new(row.password_hash)),
            None => Err(RepositoryError::NotFound),
        }
    }

    async fn get_username_by_id(&self, user_id: &UserId) -> Result<UserName, RepositoryError> {
        let record = sqlx::query!(
            r#"
                  SELECT  name
                  FROM users
                  WHERE id = $1
                  "#,
            user_id.uuid()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        match record {
            Some(row) => Ok(UserName::new(row.name)),
            None => Err(RepositoryError::NotFound),
        }
    }

    async fn get_user_id_by_email(&self, email: &Email) -> Result<UserId, RepositoryError> {
        let record = sqlx::query!(
            r#"
                SELECT id
                FROM users
                WHERE email = $1
            "#,
            email.as_str()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        match record {
            Some(row) => Ok(UserId::new(row.id)),
            None => Err(RepositoryError::NotFound),
        }
    }

    async fn confirm_registration(
        &self,
        user: &User,
        verification_code: &VerificationCode,
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let user_result = sqlx::query!(
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
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if user_result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }

        let code_result = sqlx::query!(
            r#"
                UPDATE verification_codes
                SET consumed_at = $2
                WHERE id = $1
            "#,
            verification_code.id().uuid(),
            verification_code.consumed_at(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if code_result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;

        Ok(())
    }
}
