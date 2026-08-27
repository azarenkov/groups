use async_trait::async_trait;

use identity_domain::{refresh_token::RefreshToken, user::User, user::email::Email, user::locale::Locale, user::repository::UserRepository, verification::code::VerificationCode};
use domain::{shared::outbox::outbox_record::OutboxRecord, shared::repository::error::RepositoryError, shared::value_objects::password_hash::PasswordHash, user::id::UserId, user::name::UserName};
use sqlx::{Pool, Postgres};

use crate::database::postgres::{
    dto::user::UserDto,
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
                INSERT INTO users (id, name, email, password_hash, created_at, updated_at, locale)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            user.id().uuid(),
            user.name().as_str(),
            user.email().as_str(),
            user.password_hash().as_str(),
            user.created_at(),
            user.updated_at(),
            user.locale().as_str(),
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(())
    }

    async fn register_user(
        &self,
        user: &User,
        refresh_token: &RefreshToken,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

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
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

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
                phone_verified_at,
                locale
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

    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepositoryError> {
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
                phone_verified_at,
                locale
            FROM users
            WHERE email = $1
            "#,
            email.as_str(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(dto.map(User::from))
    }

    async fn ids_by_names(
        &self,
        lowercase_names: &[String],
    ) -> Result<Vec<(UserId, UserName)>, RepositoryError> {
        if lowercase_names.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query!(
            r#"
            SELECT id, name
            FROM users
            WHERE LOWER(name COLLATE "C") = ANY($1)
            "#,
            lowercase_names,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(rows
            .into_iter()
            .map(|r| (UserId::new(r.id), UserName::new(r.name)))
            .collect())
    }

    async fn search_by_name_prefix(
        &self,
        prefix: &str,
        limit: i64,
        exclude_user: &UserId,
    ) -> Result<Vec<(UserId, UserName)>, RepositoryError> {
        let pattern = format!("{}%", prefix.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
        let rows = sqlx::query!(
            r#"
            SELECT id, name
            FROM users
            WHERE name ILIKE $1 ESCAPE '\'
              AND id <> $2
            ORDER BY name ASC
            LIMIT $3
            "#,
            pattern,
            exclude_user.uuid(),
            limit,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(rows
            .into_iter()
            .map(|r| (UserId::new(r.id), UserName::new(r.name)))
            .collect())
    }

    async fn find_by_name(&self, name: &UserName) -> Result<Option<User>, RepositoryError> {
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
                phone_verified_at,
                locale
            FROM users
            WHERE LOWER(name COLLATE "C") = LOWER($1 COLLATE "C")
            "#,
            name.as_str(),
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
        events: &[OutboxRecord],
    ) -> Result<bool, RepositoryError> {
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
                WHERE id = $1 AND consumed_at IS NULL
            "#,
            verification_code.id().uuid(),
            verification_code.consumed_at(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if code_result.rows_affected() == 0 {
            return Ok(false);
        }

        save_outbox_records(&mut tx, events).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;

        Ok(true)
    }

    async fn update_locale(
        &self,
        user_id: &UserId,
        locale: Locale,
    ) -> Result<(), RepositoryError> {
        let result = sqlx::query!(
            r#"
                UPDATE users
                SET locale = $2, updated_at = now()
                WHERE id = $1
            "#,
            user_id.uuid(),
            locale.as_str(),
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        Ok(())
    }

    async fn set_name_with_events(
        &self,
        user_id: &UserId,
        name: &UserName,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let result = sqlx::query!(
            r#"
                UPDATE users
                SET name = $2, updated_at = now()
                WHERE id = $1
            "#,
            user_id.uuid(),
            name.as_str(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }

        let committed_at = chrono::Utc::now();
        let mut stamped = events.to_vec();
        for record in &mut stamped {
            record.occurred_at = committed_at;
            if let Some(payload) = record.payload.as_object_mut() {
                payload.insert(
                    "occurred_at".to_string(),
                    serde_json::to_value(committed_at)
                        .map_err(|e| RepositoryError::InvalidData(e.to_string()))?,
                );
            }
        }

        save_outbox_records(&mut tx, &stamped).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn claim_with_oauth(
        &self,
        user_id: &UserId,
        password_hash: &PasswordHash,
        verified_at: chrono::DateTime<chrono::Utc>,
        provider: &str,
        subject: &str,
        email: &Email,
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let linked = sqlx::query(
            r#"
                INSERT INTO oauth_identities (provider, subject, user_id, email)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (provider, subject) DO NOTHING
            "#,
        )
        .bind(provider)
        .bind(subject)
        .bind(user_id.uuid())
        .bind(email.as_str())
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if linked.rows_affected() == 0 {
            return Err(RepositoryError::AlreadyExists);
        }

        let updated = sqlx::query(
            r#"
                UPDATE users
                SET password_hash = $2,
                    email_verified_at = $3,
                    updated_at = now()
                WHERE id = $1
            "#,
        )
        .bind(user_id.uuid())
        .bind(password_hash.as_str())
        .bind(verified_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if updated.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }

        sqlx::query(
            r#"
                UPDATE refresh_tokens
                SET revoked_at = now()
                WHERE user_id = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(user_id.uuid())
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn find_user_id_by_oauth(
        &self,
        provider: &str,
        subject: &str,
    ) -> Result<Option<UserId>, RepositoryError> {
        let row: Option<(uuid::Uuid,)> = sqlx::query_as(
            "SELECT user_id FROM oauth_identities WHERE provider = $1 AND subject = $2",
        )
        .bind(provider)
        .bind(subject)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(row.map(|(id,)| UserId::new(id)))
    }

    async fn find_oauth_subject_for_user(
        &self,
        provider: &str,
        user_id: &UserId,
    ) -> Result<Option<String>, RepositoryError> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT subject FROM oauth_identities WHERE provider = $1 AND user_id = $2",
        )
        .bind(provider)
        .bind(user_id.uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(row.map(|(subject,)| subject))
    }

    async fn link_oauth_identity(
        &self,
        provider: &str,
        subject: &str,
        user_id: &UserId,
        email: &Email,
    ) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"
                INSERT INTO oauth_identities (provider, subject, user_id, email)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (provider, subject) DO NOTHING
            "#,
        )
        .bind(provider)
        .bind(subject)
        .bind(user_id.uuid())
        .bind(email.as_str())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(())
    }

}
