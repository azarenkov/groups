use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use ports::{repository::RepositoryError, user_profile::PublicProfile, user_profile::UserProfileRepository};
use sqlx::{Pool, Postgres, Row};

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

pub struct PostgresUserProfileRepository {
    pool: Pool<Postgres>,
    table: &'static str,
}

impl PostgresUserProfileRepository {
    pub fn messaging(pool: Pool<Postgres>) -> Self {
        Self {
            pool,
            table: "messaging_user_profiles",
        }
    }

    pub fn voice(pool: Pool<Postgres>) -> Self {
        Self {
            pool,
            table: "voice_user_profiles",
        }
    }
}

#[async_trait]
impl UserProfileRepository for PostgresUserProfileRepository {
    async fn get_public_profile(&self, user_id: &UserId) -> Result<Option<PublicProfile>, RepositoryError> {
        let row = sqlx::query(&format!(
            "SELECT CASE WHEN active THEN name ELSE '' END, CASE WHEN active THEN bio ELSE '' END, CASE WHEN active THEN profile_updated_at ELSE GREATEST(profile_updated_at, active_updated_at) END, active, CASE WHEN active THEN timezone ELSE '' END, registered_at FROM {} WHERE user_id = $1 AND name <> ''",
            self.table
        ))
        .bind(user_id.uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map(|r| PublicProfile {
            user_id: *user_id,
            name: r.get::<String, _>(0),
            bio: r.get::<String, _>(1),
            profile_updated_at: r.get::<Option<DateTime<Utc>>, _>(2),
            active: r.get::<bool, _>(3),
            timezone: r.get::<String, _>(4),
            registered_at: r.get::<Option<DateTime<Utc>>, _>(5),
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
        sqlx::query(&format!(
            r#"
            INSERT INTO {table} (user_id, name, bio, show_activity, active, updated_at, profile_updated_at, active_updated_at, timezone)
            VALUES ($1, '', $2, $3, $5, to_timestamp(0), $4, $4, $6)
            ON CONFLICT (user_id) DO UPDATE
                SET bio = CASE WHEN {table}.profile_updated_at IS NULL OR {table}.profile_updated_at <= EXCLUDED.profile_updated_at THEN EXCLUDED.bio ELSE {table}.bio END,
                    show_activity = CASE WHEN {table}.profile_updated_at IS NULL OR {table}.profile_updated_at <= EXCLUDED.profile_updated_at THEN EXCLUDED.show_activity ELSE {table}.show_activity END,
                    timezone = CASE WHEN {table}.profile_updated_at IS NULL OR {table}.profile_updated_at <= EXCLUDED.profile_updated_at THEN EXCLUDED.timezone ELSE {table}.timezone END,
                    profile_updated_at = GREATEST({table}.profile_updated_at, EXCLUDED.profile_updated_at),
                    active = CASE WHEN {table}.active_updated_at <= EXCLUDED.active_updated_at THEN EXCLUDED.active ELSE {table}.active END,
                    active_updated_at = GREATEST({table}.active_updated_at, EXCLUDED.active_updated_at)
            "#,
            table = self.table
        ))
        .bind(user_id.uuid())
        .bind(bio)
        .bind(show_activity)
        .bind(occurred_at)
        .bind(active)
        .bind(timezone)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn set_registered_at(&self, user_id: &UserId, registered_at: DateTime<Utc>) -> Result<(), RepositoryError> {
        sqlx::query(&format!(
            r#"
            INSERT INTO {table} (user_id, name, updated_at, registered_at)
            VALUES ($1, '', to_timestamp(0), $2)
            ON CONFLICT (user_id) DO UPDATE
                SET registered_at = COALESCE({table}.registered_at, EXCLUDED.registered_at)
            "#,
            table = self.table
        ))
        .bind(user_id.uuid())
        .bind(registered_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn get_name(&self, user_id: &UserId) -> Result<Option<String>, RepositoryError> {
        let row = sqlx::query(&format!(
            "SELECT name FROM {} WHERE user_id = $1",
            self.table
        ))
        .bind(user_id.uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map(|r| r.get::<String, _>(0)))
    }

    async fn find_user_by_name(&self, name: &str) -> Result<Option<UserId>, RepositoryError> {
        let row = sqlx::query(&format!(
            r#"SELECT user_id FROM {} WHERE LOWER(name COLLATE "C") = LOWER($1 COLLATE "C") AND active"#,
            self.table
        ))
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map(|r| UserId::new(r.get::<uuid::Uuid, _>(0))))
    }

    async fn upsert_name(
        &self,
        user_id: &UserId,
        name: &str,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        sqlx::query(&format!(
            r#"
            INSERT INTO {table} (user_id, name, updated_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id) DO UPDATE
                SET name = EXCLUDED.name,
                    updated_at = GREATEST({table}.updated_at, EXCLUDED.updated_at)
                WHERE {table}.updated_at <= EXCLUDED.updated_at
                   OR ({table}.name = '' AND {table}.active)
            "#,
            table = self.table
        ))
        .bind(user_id.uuid())
        .bind(name)
        .bind(occurred_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn is_active(&self, user_id: &UserId) -> Result<bool, RepositoryError> {
        let row = sqlx::query(&format!(
            "SELECT active FROM {} WHERE user_id = $1",
            self.table
        ))
        .bind(user_id.uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map(|r| r.get::<bool, _>(0)).unwrap_or(true))
    }

    async fn is_deactivation_current(
        &self,
        user_id: &UserId,
        occurred_at: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        let row = sqlx::query(&format!(
            "SELECT active, active_updated_at FROM {} WHERE user_id = $1",
            self.table
        ))
        .bind(user_id.uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(match row {
            Some(r) => {
                let active: bool = r.get(0);
                let updated_at: DateTime<Utc> = r.get(1);
                !(active && updated_at > occurred_at)
            }
            None => true,
        })
    }

    async fn set_active(
        &self,
        user_id: &UserId,
        active: bool,
        occurred_at: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        let result = sqlx::query(&format!(
            r#"
            INSERT INTO {table} (user_id, name, active, updated_at, active_updated_at)
            VALUES ($1, '', $2, to_timestamp(0), $3)
            ON CONFLICT (user_id) DO UPDATE
                SET active = EXCLUDED.active, active_updated_at = EXCLUDED.active_updated_at
                WHERE {table}.active_updated_at <= EXCLUDED.active_updated_at
            "#,
            table = self.table
        ))
        .bind(user_id.uuid())
        .bind(active)
        .bind(occurred_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(result.rows_affected() > 0)
    }


}
