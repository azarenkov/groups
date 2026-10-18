use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use infrastructure_common::database::postgres::error::map_sqlx_error_to_domain_error;
use ports::{repository::RepositoryError, user_profile::PublicProfile, user_profile::UserProfileRepository};
use sqlx::{Pool, Postgres};

pub struct PostgresUserProfileRepository {
    pool: Pool<Postgres>,
}

impl PostgresUserProfileRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserProfileRepository for PostgresUserProfileRepository {
    async fn get_public_profile(&self, user_id: &UserId) -> Result<Option<PublicProfile>, RepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT
                CASE WHEN active THEN name ELSE '' END AS "name!",
                CASE WHEN active THEN bio ELSE '' END AS "bio!",
                CASE WHEN active THEN profile_updated_at ELSE GREATEST(profile_updated_at, active_updated_at) END AS profile_updated_at,
                active,
                CASE WHEN active THEN timezone ELSE '' END AS "timezone!",
                registered_at
            FROM messaging_user_profiles
            WHERE user_id = $1 AND name <> ''
            "#,
            user_id.uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map(|r| PublicProfile {
            user_id: *user_id,
            name: r.name,
            bio: r.bio,
            profile_updated_at: r.profile_updated_at,
            active: r.active,
            timezone: r.timezone,
            registered_at: r.registered_at,
        }))
    }

    async fn update_profile(
        &self,
        user_id: &UserId,
        bio: &str,
        show_activity: bool,
        active: bool,
        timezone: &str,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            INSERT INTO messaging_user_profiles (user_id, name, bio, show_activity, active, updated_at, profile_updated_at, active_updated_at, timezone)
            VALUES ($1, '', $2, $3, $5, to_timestamp(0), $4, $4, $6)
            ON CONFLICT (user_id) DO UPDATE
                SET bio = CASE WHEN messaging_user_profiles.profile_updated_at IS NULL OR messaging_user_profiles.profile_updated_at <= EXCLUDED.profile_updated_at THEN EXCLUDED.bio ELSE messaging_user_profiles.bio END,
                    show_activity = CASE WHEN messaging_user_profiles.profile_updated_at IS NULL OR messaging_user_profiles.profile_updated_at <= EXCLUDED.profile_updated_at THEN EXCLUDED.show_activity ELSE messaging_user_profiles.show_activity END,
                    timezone = CASE WHEN messaging_user_profiles.profile_updated_at IS NULL OR messaging_user_profiles.profile_updated_at <= EXCLUDED.profile_updated_at THEN EXCLUDED.timezone ELSE messaging_user_profiles.timezone END,
                    profile_updated_at = GREATEST(messaging_user_profiles.profile_updated_at, EXCLUDED.profile_updated_at),
                    active = CASE WHEN messaging_user_profiles.active_updated_at <= EXCLUDED.active_updated_at THEN EXCLUDED.active ELSE messaging_user_profiles.active END,
                    active_updated_at = GREATEST(messaging_user_profiles.active_updated_at, EXCLUDED.active_updated_at)
            "#,
            user_id.uuid(),
            bio,
            show_activity,
            occurred_at,
            active,
            timezone,
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn set_registered_at(&self, user_id: &UserId, registered_at: DateTime<Utc>) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            INSERT INTO messaging_user_profiles (user_id, name, updated_at, registered_at)
            VALUES ($1, '', to_timestamp(0), $2)
            ON CONFLICT (user_id) DO UPDATE
                SET registered_at = COALESCE(messaging_user_profiles.registered_at, EXCLUDED.registered_at)
            "#,
            user_id.uuid(),
            registered_at,
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn get_name(&self, user_id: &UserId) -> Result<Option<String>, RepositoryError> {
        sqlx::query_scalar!(r#"SELECT name FROM messaging_user_profiles WHERE user_id = $1"#, user_id.uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error)
    }

    async fn find_user_by_name(&self, name: &str) -> Result<Option<UserId>, RepositoryError> {
        let user_id = sqlx::query_scalar!(
            r#"SELECT user_id FROM messaging_user_profiles WHERE LOWER(name COLLATE "C") = LOWER($1::text COLLATE "C") AND active"#,
            name,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(user_id.map(UserId::new))
    }

    async fn upsert_name(
        &self,
        user_id: &UserId,
        name: &str,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            INSERT INTO messaging_user_profiles (user_id, name, updated_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id) DO UPDATE
                SET name = EXCLUDED.name,
                    updated_at = GREATEST(messaging_user_profiles.updated_at, EXCLUDED.updated_at)
                WHERE messaging_user_profiles.updated_at <= EXCLUDED.updated_at
                   OR (messaging_user_profiles.name = '' AND messaging_user_profiles.active)
            "#,
            user_id.uuid(),
            name,
            occurred_at,
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn is_active(&self, user_id: &UserId) -> Result<bool, RepositoryError> {
        let active = sqlx::query_scalar!(r#"SELECT active FROM messaging_user_profiles WHERE user_id = $1"#, user_id.uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        Ok(active.unwrap_or(true))
    }

    async fn is_deactivation_current(
        &self,
        user_id: &UserId,
        occurred_at: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        let row = sqlx::query!(r#"SELECT active, active_updated_at FROM messaging_user_profiles WHERE user_id = $1"#, user_id.uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        Ok(match row {
            Some(r) => !(r.active && r.active_updated_at > occurred_at),
            None => true,
        })
    }

    async fn set_active(
        &self,
        user_id: &UserId,
        active: bool,
        occurred_at: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        let result = sqlx::query!(
            r#"
            INSERT INTO messaging_user_profiles (user_id, name, active, updated_at, active_updated_at)
            VALUES ($1, '', $2, to_timestamp(0), $3)
            ON CONFLICT (user_id) DO UPDATE
                SET active = EXCLUDED.active, active_updated_at = EXCLUDED.active_updated_at
                WHERE messaging_user_profiles.active_updated_at <= EXCLUDED.active_updated_at
            "#,
            user_id.uuid(),
            active,
            occurred_at,
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(result.rows_affected() > 0)
    }
}
