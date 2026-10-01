use async_trait::async_trait;
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use chrono::{DateTime, Utc};
use identity_domain::{events::IdentityEvent, user::tier::Entitlements, user::tier::Tier, user::tier::TrialHistory, user::tier::trial_email_key, verification::throttle::IssuedCodes, user::email::Email, refresh_token::RefreshToken, refresh_token::id::RefreshTokenId, unit_of_work::IdentityTransaction, unit_of_work::IdentityUnitOfWork, user::User, verification::code::VerificationCode, verification::code_hash::CodeHash, verification::id::VerificationCodeId, verification::purpose::VerificationPurpose};
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
    trial_key: std::sync::Arc<[u8]>,
}

impl PostgresIdentityUnitOfWork {
    pub fn new(pool: Pool<Postgres>, trial_key: &[u8]) -> Self {
        Self {
            pool,
            trial_key: trial_key.into(),
        }
    }
}

pub struct PostgresIdentityTransaction {
    tx: Transaction<'static, Postgres>,
    trial_key: std::sync::Arc<[u8]>,
}

#[async_trait]
impl IdentityUnitOfWork for PostgresIdentityUnitOfWork {
    async fn begin(&self) -> Result<Box<dyn IdentityTransaction>, RepositoryError> {
        let tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        Ok(Box::new(PostgresIdentityTransaction {
            tx,
            trial_key: self.trial_key.clone(),
        }))
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
                   email_verified_at, phone_verified_at, locale, pending_email, password_changed_at, disabled_at, deleted_at, bio, badge_style, show_activity, show_game_activity, timezone
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
                   email_verified_at, phone_verified_at, locale, pending_email, password_changed_at, disabled_at, deleted_at, bio, badge_style, show_activity, show_game_activity, timezone
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
                    timezone = $16,
                    badge_style = $17
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
            user.badge_style().as_str(),
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

    async fn lock_user(&mut self, user_id: &UserId) -> Result<bool, RepositoryError> {
        let row = sqlx::query_scalar!(
            r#"SELECT 1 AS "one!" FROM users WHERE id = $1 FOR UPDATE"#,
            user_id.uuid(),
        )
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.is_some())
    }

    async fn load_entitlements(&mut self, user_id: &UserId) -> Result<Entitlements, RepositoryError> {
        let row = sqlx::query!(
            r#"SELECT tier, valid_until, member_number, source, trial_used_at FROM user_entitlements WHERE user_id = $1"#,
            user_id.uuid(),
        )
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row
            .map(|r| Entitlements {
                tier: Tier::parse(&r.tier).unwrap_or_default(),
                valid_until: r.valid_until,
                member_number: r.member_number,
                trial: r.source == "trial",
                trial_used_at: r.trial_used_at,
            })
            .unwrap_or_default())
    }

    async fn save_entitlements(
        &mut self,
        user_id: &UserId,
        tier: &str,
        valid_until: Option<DateTime<Utc>>,
        source: &str,
    ) -> Result<Option<i64>, RepositoryError> {
        let current = sqlx::query_scalar!(
            r#"
            INSERT INTO user_entitlements (user_id, tier, valid_until, source, updated_at)
            VALUES ($1, $2, $3, $4, now())
            ON CONFLICT (user_id) DO UPDATE
                SET tier = EXCLUDED.tier,
                    valid_until = EXCLUDED.valid_until,
                    source = EXCLUDED.source,
                    updated_at = now()
            RETURNING member_number
            "#,
            user_id.uuid(),
            tier,
            valid_until,
            source,
        )
        .fetch_one(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if current.is_some() || tier == "free" || source == "trial" {
            return Ok(current);
        }
        sqlx::query_scalar!(
            r#"
            UPDATE user_entitlements
            SET member_number = nextval('pro_member_number_seq')
            WHERE user_id = $1
            RETURNING member_number
            "#,
            user_id.uuid(),
        )
        .fetch_one(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)
    }

    async fn issued_verification_codes(&mut self, user_id: &UserId, purpose: VerificationPurpose, since: DateTime<Utc>) -> Result<IssuedCodes, RepositoryError> {
        let purpose = VerificationPurposeDto::from(&purpose);
        let row = sqlx::query!(
            r#"
            SELECT MAX(created_at) AS last_issued_at, COUNT(*) FILTER (WHERE created_at > $3) AS "issued_last_hour!"
            FROM verification_codes
            WHERE user_id = $1 AND purpose = $2
            "#,
            user_id.uuid(),
            purpose as VerificationPurposeDto,
            since,
        )
        .fetch_one(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(IssuedCodes {
            last_issued_at: row.last_issued_at,
            issued_last_hour: row.issued_last_hour,
        })
    }

    async fn trial_history(&mut self, email: &Email, ip: Option<&str>, since: DateTime<Utc>) -> Result<TrialHistory, RepositoryError> {
        let email_hash = trial_hash(&self.trial_key, &trial_email_key(email.as_str()));
        let ip_hash = ip.map(|ip| trial_hash(&self.trial_key, ip));
        for key in std::iter::once(format!("trial:email:{email_hash}"))
            .chain(ip_hash.as_ref().map(|hash| format!("trial:ip:{hash}")))
        {
            sqlx::query!("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))", key)
                .execute(&mut *self.tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
        }
        let email_used = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM trial_grants WHERE email_hash = $1) AS "used!""#,
            email_hash,
        )
        .fetch_one(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let recent_from_ip = match ip_hash {
            Some(ip_hash) => sqlx::query_scalar!(
                r#"SELECT COUNT(*) AS "count!" FROM trial_grants WHERE ip_hash = $1 AND granted_at > $2"#,
                ip_hash,
                since,
            )
            .fetch_one(&mut *self.tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?,
            None => 0,
        };
        Ok(TrialHistory { email_used, recent_from_ip })
    }

    async fn start_trial(&mut self, user_id: &UserId, email: &Email, ip: Option<&str>, valid_until: DateTime<Utc>, now: DateTime<Utc>) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            INSERT INTO user_entitlements (user_id, tier, valid_until, source, trial_used_at, updated_at)
            VALUES ($1, 'pro', $2, 'trial', $3, now())
            ON CONFLICT (user_id) DO UPDATE
                SET tier = 'pro',
                    valid_until = EXCLUDED.valid_until,
                    source = 'trial',
                    trial_used_at = EXCLUDED.trial_used_at,
                    updated_at = now()
            "#,
            user_id.uuid(),
            valid_until,
            now,
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        sqlx::query!(
            r#"INSERT INTO trial_grants (user_id, email_hash, ip_hash, granted_at) VALUES ($1, $2, $3, $4)"#,
            user_id.uuid(),
            trial_hash(&self.trial_key, &trial_email_key(email.as_str())),
            ip.map(|ip| trial_hash(&self.trial_key, ip)),
            now,
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn load_trials_ending_for_update(&mut self, before: DateTime<Utc>, limit: i64) -> Result<Vec<UserId>, RepositoryError> {
        let ids = sqlx::query_scalar!(
            r#"
            SELECT user_id FROM user_entitlements
            WHERE source = 'trial' AND tier <> 'free' AND trial_reminded_at IS NULL
              AND valid_until IS NOT NULL AND valid_until > now() AND valid_until <= $1
            ORDER BY valid_until
            LIMIT $2
            FOR UPDATE SKIP LOCKED
            "#,
            before,
            limit,
        )
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(ids.into_iter().map(UserId::new).collect())
    }

    async fn mark_trial_reminded(&mut self, user_id: &UserId, now: DateTime<Utc>) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"UPDATE user_entitlements SET trial_reminded_at = $2 WHERE user_id = $1"#,
            user_id.uuid(),
            now,
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn load_expired_entitlements_for_update(&mut self, now: DateTime<Utc>, limit: i64) -> Result<Vec<UserId>, RepositoryError> {
        let ids = sqlx::query_scalar!(
            r#"
            SELECT user_id FROM user_entitlements
            WHERE tier <> 'free' AND valid_until IS NOT NULL AND valid_until <= $1
            ORDER BY valid_until
            LIMIT $2
            FOR UPDATE SKIP LOCKED
            "#,
            now,
            limit,
        )
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(ids.into_iter().map(UserId::new).collect())
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
        let row: Option<(i16, i16, String, bool, i16, bool, String, String, Option<String>)> = sqlx::query_as(
            "SELECT mic_volume, output_volume, noise_mode, auto_sensitivity, sensitivity, push_to_talk, ptt_key, stream_quality, screen_content FROM voice_settings WHERE user_id = $1",
        )
        .bind(user_id.uuid())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map(|r| VoiceSettings::restore(r.0, r.1, &r.2, r.3, r.4, r.5, r.6, &r.7, r.8.as_deref())))
    }

    async fn save_voice_settings(
        &mut self,
        user_id: &UserId,
        settings: &VoiceSettings,
    ) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"
                INSERT INTO voice_settings (user_id, mic_volume, output_volume, noise_mode, auto_sensitivity, sensitivity, push_to_talk, ptt_key, stream_quality, screen_content, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, now())
                ON CONFLICT (user_id) DO UPDATE
                    SET mic_volume = EXCLUDED.mic_volume,
                        output_volume = EXCLUDED.output_volume,
                        noise_mode = EXCLUDED.noise_mode,
                        auto_sensitivity = EXCLUDED.auto_sensitivity,
                        sensitivity = EXCLUDED.sensitivity,
                        push_to_talk = EXCLUDED.push_to_talk,
                        ptt_key = EXCLUDED.ptt_key,
                        stream_quality = EXCLUDED.stream_quality,
                        screen_content = EXCLUDED.screen_content,
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
        .bind(settings.stream_quality().as_str())
        .bind(settings.screen_content().map(|c| c.as_str()))
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

fn trial_hash(key: &[u8], value: &str) -> String {
    use hmac::{Hmac, Mac};
    let mut mac = <Hmac<sha2::Sha256> as Mac>::new_from_slice(key).expect("HMAC accepts keys of any length");
    mac.update(b"rift-trial-grant:");
    mac.update(value.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}
