use async_trait::async_trait;
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use chrono::{DateTime, Utc};
use identity_domain::{events::IdentityEvent, user::email::Email, refresh_token::RefreshToken, refresh_token::id::RefreshTokenId, unit_of_work::IdentityTransaction, unit_of_work::IdentityUnitOfWork, user::User, verification::code::VerificationCode, verification::code_hash::CodeHash, verification::id::VerificationCodeId, verification::purpose::VerificationPurpose};
use sqlx::{Pool, Postgres, Transaction};
use std::collections::BTreeMap;
use identity_domain::user::settings::{
    appearance::AppearanceSettings, hotkeys::HotkeyBindings,
    notification::NotificationSettings, voice::VoiceSettings,
};

use crate::database::postgres::repositories::user::restore_hotkey_bindings;

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
                   email_verified_at, phone_verified_at, locale, pending_email, password_changed_at, disabled_at, deleted_at, bio, show_activity, show_game_activity, timezone
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

    async fn load_stale_unverified_users_for_update(
        &mut self,
        registered_before: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<User>, RepositoryError> {
        let dtos = sqlx::query_as!(
            UserDto,
            r#"
            SELECT id, name, email, password_hash, created_at, updated_at,
                   email_verified_at, phone_verified_at, locale, pending_email, password_changed_at, disabled_at, deleted_at, bio, show_activity, show_game_activity, timezone
            FROM users
            WHERE email_verified_at IS NULL
              AND phone_verified_at IS NULL
              AND deleted_at IS NULL
              AND created_at < $1
            ORDER BY created_at
            LIMIT $2
            FOR UPDATE SKIP LOCKED
            "#,
            registered_before,
            limit,
        )
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(dtos.into_iter().map(User::from).collect())
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

    async fn link_oauth_identity(&mut self, provider: &str, subject: &str, user_id: &UserId, email: &Email) -> Result<bool, RepositoryError> {
        let result = sqlx::query!(
            r#"
                INSERT INTO oauth_identities (provider, subject, user_id, email)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (provider, subject) DO NOTHING
            "#,
            provider,
            subject,
            user_id.uuid(),
            email.as_str(),
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(result.rows_affected() > 0)
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
                    deleted_at = $12,
                    bio = $13,
                    show_activity = $14,
                    show_game_activity = $15,
                    timezone = $16
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
            user.bio(),
            user.show_activity(),
            user.show_game_activity(),
            user.timezone(),
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

    async fn load_appearance_for_update(
        &mut self,
        user_id: &UserId,
    ) -> Result<Option<AppearanceSettings>, RepositoryError> {
        self.lock_settings(user_id).await?;
        let row: Option<(String, String, i16, bool)> = sqlx::query_as(
            "SELECT theme, accent, chat_font_size, reduce_motion FROM user_settings WHERE user_id = $1",
        )
        .bind(user_id.uuid())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map(|r| AppearanceSettings::restore(&r.0, r.1, r.2, r.3)))
    }

    async fn save_appearance(
        &mut self,
        user_id: &UserId,
        settings: &AppearanceSettings,
    ) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"
                INSERT INTO user_settings (user_id, theme, accent, chat_font_size, reduce_motion, updated_at)
                VALUES ($1, $2, $3, $4, $5, now())
                ON CONFLICT (user_id) DO UPDATE
                    SET theme = EXCLUDED.theme,
                        accent = EXCLUDED.accent,
                        chat_font_size = EXCLUDED.chat_font_size,
                        reduce_motion = EXCLUDED.reduce_motion,
                        updated_at = now()
            "#,
        )
        .bind(user_id.uuid())
        .bind(settings.theme().as_str())
        .bind(settings.accent())
        .bind(settings.chat_font_size())
        .bind(settings.reduce_motion())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn load_voice_settings_for_update(
        &mut self,
        user_id: &UserId,
    ) -> Result<Option<VoiceSettings>, RepositoryError> {
        self.lock_settings(user_id).await?;
        let row: Option<(i16, i16, String, bool, i16, bool, String)> = sqlx::query_as(
            "SELECT mic_volume, output_volume, noise_mode, auto_sensitivity, sensitivity, push_to_talk, ptt_key FROM voice_settings WHERE user_id = $1",
        )
        .bind(user_id.uuid())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map(|r| VoiceSettings::restore(r.0, r.1, &r.2, r.3, r.4, r.5, r.6)))
    }

    async fn save_voice_settings(
        &mut self,
        user_id: &UserId,
        settings: &VoiceSettings,
    ) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"
                INSERT INTO voice_settings (user_id, mic_volume, output_volume, noise_mode, auto_sensitivity, sensitivity, push_to_talk, ptt_key, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now())
                ON CONFLICT (user_id) DO UPDATE
                    SET mic_volume = EXCLUDED.mic_volume,
                        output_volume = EXCLUDED.output_volume,
                        noise_mode = EXCLUDED.noise_mode,
                        auto_sensitivity = EXCLUDED.auto_sensitivity,
                        sensitivity = EXCLUDED.sensitivity,
                        push_to_talk = EXCLUDED.push_to_talk,
                        ptt_key = EXCLUDED.ptt_key,
                        updated_at = now()
            "#,
        )
        .bind(user_id.uuid())
        .bind(settings.mic_volume())
        .bind(settings.output_volume())
        .bind(settings.noise_mode().as_str())
        .bind(settings.auto_sensitivity())
        .bind(settings.sensitivity())
        .bind(settings.push_to_talk())
        .bind(settings.ptt_key().as_str())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn load_notification_settings_for_update(
        &mut self,
        user_id: &UserId,
    ) -> Result<Option<NotificationSettings>, RepositoryError> {
        self.lock_settings(user_id).await?;
        let row: Option<(bool, bool, String, bool, String, String, bool)> = sqlx::query_as(
            "SELECT desktop_enabled, sounds_enabled, mentions, quiet_hours_enabled, quiet_start, quiet_end, calls_enabled FROM notification_settings WHERE user_id = $1",
        )
        .bind(user_id.uuid())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map(|r| NotificationSettings::restore(r.0, r.1, &r.2, r.3, r.4, r.5, r.6)))
    }

    async fn save_notification_settings(
        &mut self,
        user_id: &UserId,
        settings: &NotificationSettings,
    ) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"
                INSERT INTO notification_settings (user_id, desktop_enabled, sounds_enabled, mentions, quiet_hours_enabled, quiet_start, quiet_end, calls_enabled, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now())
                ON CONFLICT (user_id) DO UPDATE
                    SET desktop_enabled = EXCLUDED.desktop_enabled,
                        sounds_enabled = EXCLUDED.sounds_enabled,
                        mentions = EXCLUDED.mentions,
                        quiet_hours_enabled = EXCLUDED.quiet_hours_enabled,
                        quiet_start = EXCLUDED.quiet_start,
                        quiet_end = EXCLUDED.quiet_end,
                        calls_enabled = EXCLUDED.calls_enabled,
                        updated_at = now()
            "#,
        )
        .bind(user_id.uuid())
        .bind(settings.desktop_enabled())
        .bind(settings.sounds_enabled())
        .bind(settings.mentions().as_str())
        .bind(settings.quiet_hours_enabled())
        .bind(settings.quiet_start().as_str())
        .bind(settings.quiet_end().as_str())
        .bind(settings.calls_enabled())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn load_hotkey_settings_for_update(
        &mut self,
        user_id: &UserId,
    ) -> Result<Option<HotkeyBindings>, RepositoryError> {
        self.lock_settings(user_id).await?;
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT bindings::text FROM hotkey_settings WHERE user_id = $1",
        )
        .bind(user_id.uuid())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map(|r| restore_hotkey_bindings(&r.0)))
    }

    async fn save_hotkey_settings(
        &mut self,
        user_id: &UserId,
        settings: &HotkeyBindings,
    ) -> Result<(), RepositoryError> {
        let stored: BTreeMap<&str, &str> = settings
            .overrides()
            .iter()
            .map(|(action, binding)| (action.id(), binding.as_str()))
            .collect();
        let payload = serde_json::to_string(&stored)
            .map_err(|e| RepositoryError::InvalidData(e.to_string()))?;
        sqlx::query(
            r#"
                INSERT INTO hotkey_settings (user_id, bindings, updated_at)
                VALUES ($1, $2::jsonb, now())
                ON CONFLICT (user_id) DO UPDATE
                    SET bindings = EXCLUDED.bindings,
                        updated_at = now()
            "#,
        )
        .bind(user_id.uuid())
        .bind(payload)
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn commit(self: Box<Self>) -> Result<(), RepositoryError> {
        self.tx.commit().await.map_err(map_sqlx_error_to_domain_error)
    }
}

impl PostgresIdentityTransaction {
    async fn lock_settings(&mut self, user_id: &UserId) -> Result<(), RepositoryError> {
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))")
            .bind(user_id.uuid())
            .execute(&mut *self.tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }
}
