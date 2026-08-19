use async_trait::async_trait;
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

#[async_trait]
impl FriendEdgeRepository for PostgresFriendEdgeRepository {
    async fn are_friends(&self, a: &UserId, b: &UserId) -> Result<bool, RepositoryError> {
        let row = sqlx::query(&format!(
            "SELECT EXISTS(SELECT 1 FROM {} WHERE user_id = $1 AND friend_id = $2) AS present",
            self.table
        ))
        .bind(a.uuid())
        .bind(b.uuid())
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.get::<bool, _>(0))
    }

    async fn add_edge(&self, a: &UserId, b: &UserId) -> Result<(), RepositoryError> {
        sqlx::query(&format!(
            "INSERT INTO {} (user_id, friend_id) VALUES ($1, $2), ($2, $1) ON CONFLICT DO NOTHING",
            self.table
        ))
        .bind(a.uuid())
        .bind(b.uuid())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn remove_edge(&self, a: &UserId, b: &UserId) -> Result<(), RepositoryError> {
        sqlx::query(&format!(
            "DELETE FROM {} WHERE (user_id = $1 AND friend_id = $2) OR (user_id = $2 AND friend_id = $1)",
            self.table
        ))
        .bind(a.uuid())
        .bind(b.uuid())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }
}
