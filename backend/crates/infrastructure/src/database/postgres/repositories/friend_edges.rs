use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::{
    friend_edges::FriendEdgeRepository, shared::repository::error::RepositoryError,
    user::id::UserId,
};
use sqlx::{Pool, Postgres, Row};

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

pub struct PostgresFriendEdgeRepository {
    pool: Pool<Postgres>,
    table: &'static str,
}

impl PostgresFriendEdgeRepository {
    pub fn voice(pool: Pool<Postgres>) -> Self {
        Self {
            pool,
            table: "voice_friend_edges",
        }
    }
}

impl PostgresFriendEdgeRepository {
    async fn upsert_edge(
        &self,
        a: &UserId,
        b: &UserId,
        removed: bool,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        sqlx::query(&format!(
            r#"
            INSERT INTO {table} (user_id, friend_id, removed, updated_at)
            VALUES ($1, $2, $3, $4), ($2, $1, $3, $4)
            ON CONFLICT (user_id, friend_id) DO UPDATE
                SET removed = EXCLUDED.removed, updated_at = EXCLUDED.updated_at
                WHERE {table}.updated_at <= EXCLUDED.updated_at
            "#,
            table = self.table
        ))
        .bind(a.uuid())
        .bind(b.uuid())
        .bind(removed)
        .bind(occurred_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }
}

#[async_trait]
impl FriendEdgeRepository for PostgresFriendEdgeRepository {
    async fn are_friends(&self, a: &UserId, b: &UserId) -> Result<bool, RepositoryError> {
        let row = sqlx::query(&format!(
            "SELECT EXISTS(SELECT 1 FROM {} WHERE user_id = $1 AND friend_id = $2 AND NOT removed) AS present",
            self.table
        ))
        .bind(a.uuid())
        .bind(b.uuid())
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.get::<bool, _>(0))
    }

    async fn add_edge(
        &self,
        a: &UserId,
        b: &UserId,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        self.upsert_edge(a, b, false, occurred_at).await
    }

    async fn remove_edge(
        &self,
        a: &UserId,
        b: &UserId,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        self.upsert_edge(a, b, true, occurred_at).await
    }
}
