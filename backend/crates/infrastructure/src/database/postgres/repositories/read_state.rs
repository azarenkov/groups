use async_trait::async_trait;
use domain::{
    channel::id::ChannelId,
    read_state::repository::{ChannelUnread, DmUnread, ReadStateRepository},
    shared::repository::error::RepositoryError,
    user::id::UserId,
};
use sqlx::{Pool, Postgres};

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

pub struct PostgresReadStateRepository {
    pool: Pool<Postgres>,
}

impl PostgresReadStateRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReadStateRepository for PostgresReadStateRepository {
    async fn mark_channel_read(
        &self,
        user_id: &UserId,
        channel_id: &ChannelId,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            INSERT INTO channel_read_state (user_id, channel_id, last_read_at)
            VALUES ($1, $2, now())
            ON CONFLICT (user_id, channel_id)
            DO UPDATE SET last_read_at = EXCLUDED.last_read_at
            "#,
            user_id.uuid(),
            channel_id.uuid(),
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn mark_dm_read(
        &self,
        user_id: &UserId,
        friend_id: &UserId,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            INSERT INTO dm_read_state (user_id, friend_id, last_read_at)
            VALUES ($1, $2, now())
            ON CONFLICT (user_id, friend_id)
            DO UPDATE SET last_read_at = EXCLUDED.last_read_at
            "#,
            user_id.uuid(),
            friend_id.uuid(),
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn channel_unread_counts(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<ChannelUnread>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT m.channel_id, c.server_id, COUNT(*) AS "count!"
            FROM messages m
            INNER JOIN channels c ON c.id = m.channel_id
            INNER JOIN channel_text_participants p
                ON p.channel_id = m.channel_id AND p.user_id = $1
            LEFT JOIN channel_read_state r
                ON r.channel_id = m.channel_id AND r.user_id = $1
            WHERE m.created_at > COALESCE(r.last_read_at, p.joined_at)
              AND m.author_id <> $1
            GROUP BY m.channel_id, c.server_id
            "#,
            user_id.uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(rows
            .into_iter()
            .map(|r| ChannelUnread {
                channel_id: r.channel_id,
                server_id: r.server_id,
                count: r.count,
            })
            .collect())
    }

    async fn dm_unread_counts(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<DmUnread>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT
                m.sender_id AS friend_id,
                COUNT(*) AS "count!"
            FROM dm_messages m
            INNER JOIN friendships f
                ON f.user_id = $1 AND f.friend_id = m.sender_id AND f.status = 'accepted'
            LEFT JOIN dm_read_state r
                ON r.user_id = $1 AND r.friend_id = m.sender_id
            WHERE m.recipient_id = $1
              AND m.created_at > COALESCE(r.last_read_at, '1970-01-01T00:00:00Z'::timestamptz)
            GROUP BY m.sender_id
            "#,
            user_id.uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(rows
            .into_iter()
            .map(|r| DmUnread {
                friend_id: r.friend_id,
                count: r.count,
            })
            .collect())
    }
}
