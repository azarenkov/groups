use async_trait::async_trait;
use domain::{
    shared::repository::error::RepositoryError, user::id::UserId,
    user_profile::UserProfileRepository,
};
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
            r#"SELECT user_id FROM {} WHERE LOWER(name COLLATE "C") = LOWER($1 COLLATE "C")"#,
            self.table
        ))
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map(|r| UserId::new(r.get::<uuid::Uuid, _>(0))))
    }

    async fn upsert_name(&self, user_id: &UserId, name: &str) -> Result<(), RepositoryError> {
        sqlx::query(&format!(
            r#"
            INSERT INTO {} (user_id, name)
            VALUES ($1, $2)
            ON CONFLICT (user_id) DO UPDATE SET name = EXCLUDED.name
            "#,
            self.table
        ))
        .bind(user_id.uuid())
        .bind(name)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }
}
