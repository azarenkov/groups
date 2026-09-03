use chrono::{DateTime, Utc};
use async_trait::async_trait;

use identity_domain::{user::User, user::email::Email, user::repository::UserRepository};
use domain::{password_hash::PasswordHash, user::id::UserId, user::name::UserName};
use ports::repository::RepositoryError;
use sqlx::{Pool, Postgres};

use crate::database::postgres::{
    dto::user::UserDto,
    error::map_sqlx_error_to_domain_error,
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
                locale, pending_email, password_changed_at, disabled_at, deleted_at, bio, show_activity, timezone
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
                locale, pending_email, password_changed_at, disabled_at, deleted_at, bio, show_activity, timezone
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
              AND disabled_at IS NULL
              AND deleted_at IS NULL
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
                locale, pending_email, password_changed_at, disabled_at, deleted_at, bio, show_activity, timezone
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


    async fn list_oauth_connections(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<(String, String, Option<String>)>, RepositoryError> {
        let rows: Vec<(String, String, Option<String>)> = sqlx::query_as(
            "SELECT provider, subject, display_name FROM oauth_identities WHERE user_id = $1 ORDER BY provider",
        )
        .bind(user_id.uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows)
    }

    async fn link_connection(
        &self,
        provider: &str,
        subject: &str,
        user_id: &UserId,
        display_name: &str,
    ) -> Result<bool, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        sqlx::query("DELETE FROM oauth_identities WHERE provider = $1 AND user_id = $2")
            .bind(provider)
            .bind(user_id.uuid())
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        let inserted = sqlx::query(
            r#"
                INSERT INTO oauth_identities (provider, subject, user_id, email, display_name)
                VALUES ($1, $2, $3, '', $4)
                ON CONFLICT (provider, subject) DO NOTHING
            "#,
        )
        .bind(provider)
        .bind(subject)
        .bind(user_id.uuid())
        .bind(display_name)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if inserted.rows_affected() == 0 {
            tx.rollback().await.map_err(map_sqlx_error_to_domain_error)?;
            return Ok(false);
        }
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(true)
    }

    async fn unlink_connection(
        &self,
        user_id: &UserId,
        provider: &str,
    ) -> Result<bool, RepositoryError> {
        let deleted =
            sqlx::query("DELETE FROM oauth_identities WHERE provider = $1 AND user_id = $2")
                .bind(provider)
                .bind(user_id.uuid())
                .execute(&self.pool)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
        Ok(deleted.rows_affected() > 0)
    }

    async fn create_connection_state(
        &self,
        state: uuid::Uuid,
        user_id: &UserId,
        provider: &str,
    ) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM connection_states WHERE created_at < now() - interval '10 minutes'")
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        sqlx::query("INSERT INTO connection_states (state, user_id, provider) VALUES ($1, $2, $3)")
            .bind(state)
            .bind(user_id.uuid())
            .bind(provider)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn consume_connection_state(
        &self,
        state: uuid::Uuid,
    ) -> Result<Option<(UserId, String, DateTime<Utc>)>, RepositoryError> {
        let row: Option<(uuid::Uuid, String, DateTime<Utc>)> = sqlx::query_as(
            "DELETE FROM connection_states WHERE state = $1 RETURNING user_id, provider, created_at",
        )
        .bind(state)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map(|(id, provider, created_at)| (UserId::new(id), provider, created_at)))
    }

    async fn get_appearance(
        &self,
        user_id: &UserId,
    ) -> Result<Option<(String, String, i16, bool)>, RepositoryError> {
        let row: Option<(String, String, i16, bool)> = sqlx::query_as(
            "SELECT theme, accent, chat_font_size, reduce_motion FROM user_settings WHERE user_id = $1",
        )
        .bind(user_id.uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row)
    }

    async fn apply_appearance_patch(
        &self,
        user_id: &UserId,
        theme: Option<&str>,
        accent: Option<&str>,
        chat_font_size: Option<i16>,
        reduce_motion: Option<bool>,
    ) -> Result<(String, String, i16, bool), RepositoryError> {
        let row: (String, String, i16, bool) = sqlx::query_as(
            r#"
                INSERT INTO user_settings (user_id, theme, accent, chat_font_size, reduce_motion, updated_at)
                VALUES ($1, COALESCE($2, 'system'), COALESCE($3, ''), COALESCE($4, 14), COALESCE($5, false), now())
                ON CONFLICT (user_id) DO UPDATE
                    SET theme = COALESCE($2, user_settings.theme),
                        accent = COALESCE($3, user_settings.accent),
                        chat_font_size = COALESCE($4, user_settings.chat_font_size),
                        reduce_motion = COALESCE($5, user_settings.reduce_motion),
                        updated_at = now()
                RETURNING theme, accent, chat_font_size, reduce_motion
            "#,
        )
        .bind(user_id.uuid())
        .bind(theme)
        .bind(accent)
        .bind(chat_font_size)
        .bind(reduce_motion)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row)
    }

    async fn get_voice_settings(
        &self,
        user_id: &UserId,
    ) -> Result<Option<(i16, i16, String, bool, i16, bool, String)>, RepositoryError> {
        let row: Option<(i16, i16, String, bool, i16, bool, String)> = sqlx::query_as(
            "SELECT mic_volume, output_volume, noise_mode, auto_sensitivity, sensitivity, push_to_talk, ptt_key FROM voice_settings WHERE user_id = $1",
        )
        .bind(user_id.uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row)
    }

    async fn apply_voice_settings_patch(
        &self,
        user_id: &UserId,
        mic_volume: Option<i16>,
        output_volume: Option<i16>,
        noise_mode: Option<&str>,
        auto_sensitivity: Option<bool>,
        sensitivity: Option<i16>,
        push_to_talk: Option<bool>,
        ptt_key: Option<&str>,
    ) -> Result<(i16, i16, String, bool, i16, bool, String), RepositoryError> {
        let row: (i16, i16, String, bool, i16, bool, String) = sqlx::query_as(
            r#"
                INSERT INTO voice_settings (user_id, mic_volume, output_volume, noise_mode, auto_sensitivity, sensitivity, push_to_talk, ptt_key, updated_at)
                VALUES ($1, COALESCE($2, 100), COALESCE($3, 100), COALESCE($4, 'standard'), COALESCE($5, true), COALESCE($6, 50), COALESCE($7, false), COALESCE($8, 'KeyM'), now())
                ON CONFLICT (user_id) DO UPDATE
                    SET mic_volume = COALESCE($2, voice_settings.mic_volume),
                        output_volume = COALESCE($3, voice_settings.output_volume),
                        noise_mode = COALESCE($4, voice_settings.noise_mode),
                        auto_sensitivity = COALESCE($5, voice_settings.auto_sensitivity),
                        sensitivity = COALESCE($6, voice_settings.sensitivity),
                        push_to_talk = COALESCE($7, voice_settings.push_to_talk),
                        ptt_key = COALESCE($8, voice_settings.ptt_key),
                        updated_at = now()
                RETURNING mic_volume, output_volume, noise_mode, auto_sensitivity, sensitivity, push_to_talk, ptt_key
            "#,
        )
        .bind(user_id.uuid())
        .bind(mic_volume)
        .bind(output_volume)
        .bind(noise_mode)
        .bind(auto_sensitivity)
        .bind(sensitivity)
        .bind(push_to_talk)
        .bind(ptt_key)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row)
    }
}
