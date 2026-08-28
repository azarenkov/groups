use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use ports::{repository::RepositoryError, user_profile::UserProfileRepository};
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
                   OR {table}.name = ''
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

    async fn set_active(
        &self,
        user_id: &UserId,
        active: bool,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        sqlx::query(&format!(
            r#"
            INSERT INTO {table} (user_id, name, active, updated_at)
            VALUES ($1, '', $2, $3)
            ON CONFLICT (user_id) DO UPDATE
                SET active = EXCLUDED.active, updated_at = EXCLUDED.updated_at
                WHERE {table}.updated_at <= EXCLUDED.updated_at
            "#,
            table = self.table
        ))
        .bind(user_id.uuid())
        .bind(active)
        .bind(occurred_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }
}
