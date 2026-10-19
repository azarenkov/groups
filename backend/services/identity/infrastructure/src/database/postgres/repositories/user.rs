use std::collections::BTreeMap;
use identity_domain::user::settings::{
    appearance::AppearanceSettings, hotkeys::HotkeyBindings,
    notification::NotificationSettings, voice::VoiceSettings,
};
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
                INSERT INTO users (id, name, email, password_hash, created_at, updated_at, locale, role)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            user.id().uuid(),
            user.name().as_str(),
            user.email().as_str(),
            user.password_hash().as_str(),
            user.created_at(),
            user.updated_at(),
            user.locale().as_str(),
            user.role().as_str(),
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
                locale, pending_email, password_changed_at, disabled_at, deleted_at, bio, show_activity, timezone, emoji_status_sticker_id, emoji_status_until, role
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
                locale, pending_email, password_changed_at, disabled_at, deleted_at, bio, show_activity, timezone, emoji_status_sticker_id, emoji_status_until, role
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

    async fn find_by_email_ignoring_case(&self, email: &Email) -> Result<Option<User>, RepositoryError> {
        let dtos = sqlx::query_as!(
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
                locale, pending_email, password_changed_at, disabled_at, deleted_at, bio, show_activity, timezone, emoji_status_sticker_id, emoji_status_until, role
            FROM users
            WHERE lower(email) = lower($1)
            LIMIT 2
            "#,
            email.as_str(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if dtos.len() != 1 {
            return Ok(None);
        }
        Ok(dtos.into_iter().next().map(User::from))
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
                locale, pending_email, password_changed_at, disabled_at, deleted_at, bio, show_activity, timezone, emoji_status_sticker_id, emoji_status_until, role
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


    async fn get_appearance(
        &self,
        user_id: &UserId,
    ) -> Result<Option<AppearanceSettings>, RepositoryError> {
        let row: Option<(String, String, i16, bool)> = sqlx::query_as(
            "SELECT theme, accent, chat_font_size, reduce_motion FROM user_settings WHERE user_id = $1",
        )
        .bind(user_id.uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map(|r| AppearanceSettings::restore(&r.0, r.1, r.2, r.3)))
    }

    async fn get_voice_settings(
        &self,
        user_id: &UserId,
    ) -> Result<Option<VoiceSettings>, RepositoryError> {
        let row: Option<(i16, i16, String, bool, i16, bool, String, String, Option<String>)> = sqlx::query_as(
            "SELECT mic_volume, output_volume, noise_mode, auto_sensitivity, sensitivity, push_to_talk, ptt_key, stream_quality, screen_content FROM voice_settings WHERE user_id = $1",
        )
        .bind(user_id.uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map(|r| VoiceSettings::restore(r.0, r.1, &r.2, r.3, r.4, r.5, r.6, &r.7, r.8.as_deref())))
    }

    async fn get_notification_settings(
        &self,
        user_id: &UserId,
    ) -> Result<Option<NotificationSettings>, RepositoryError> {
        let row: Option<(bool, bool, String, bool, String, String, bool, bool)> = sqlx::query_as(
            "SELECT desktop_enabled, sounds_enabled, mentions, quiet_hours_enabled, quiet_start, quiet_end, calls_enabled, voice_cues_enabled FROM notification_settings WHERE user_id = $1",
        )
        .bind(user_id.uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map(|r| NotificationSettings::restore(r.0, r.1, &r.2, r.3, r.4, r.5, r.6, r.7)))
    }

    async fn appear_offline_among(&self, user_ids: &[UserId]) -> Result<Vec<UserId>, RepositoryError> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let ids: Vec<uuid::Uuid> = user_ids.iter().map(|id| id.uuid()).collect();
        let rows: Vec<(uuid::Uuid,)> = sqlx::query_as(
            "SELECT user_id FROM user_presence_preferences WHERE user_id = ANY($1) AND appear_offline",
        )
        .bind(&ids)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows.into_iter().map(|(id,)| UserId::new(id)).collect())
    }

    async fn get_hotkey_settings(
        &self,
        user_id: &UserId,
    ) -> Result<Option<HotkeyBindings>, RepositoryError> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT bindings::text FROM hotkey_settings WHERE user_id = $1",
        )
        .bind(user_id.uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map(|r| restore_hotkey_bindings(&r.0)))
    }
}

pub fn restore_hotkey_bindings(raw: &str) -> HotkeyBindings {
    let stored: BTreeMap<String, serde_json::Value> = serde_json::from_str(raw).unwrap_or_default();
    HotkeyBindings::restore(
        stored
            .into_iter()
            .filter_map(|(action, value)| match value {
                serde_json::Value::String(binding) => Some((action, binding)),
                _ => None,
            })
            .collect(),
    )
}

