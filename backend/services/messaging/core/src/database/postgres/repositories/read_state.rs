use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::{
    channel::id::ChannelId,
    read_state::repository::{ChannelUnread, DmUnread, MemberRead, ReadStateRepository},
    shared::{
        outbox::outbox_record::{AggregateType, OutboxRecord},
        repository::error::RepositoryError,
    },
    user::id::UserId,
};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::database::postgres::{
    error::map_sqlx_error_to_domain_error, repositories::outbox_record::save_outbox_records,
};

pub struct PostgresReadStateRepository {
    pool: Pool<Postgres>,
}

impl PostgresReadStateRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

fn read_event(
    aggregate_id: Uuid,
    event_type: &str,
    payload: serde_json::Value,
    occurred_at: DateTime<Utc>,
) -> OutboxRecord {
    OutboxRecord {
        id: Uuid::new_v4(),
        aggregate_id,
        aggregate_type: AggregateType::User,
        parent_id: None,
        parent_type: None,
        event_type: event_type.to_string(),
        payload,
        occurred_at,
        request_id: None,
    }
}

#[async_trait]
impl ReadStateRepository for PostgresReadStateRepository {
    async fn mark_channel_read(
        &self,
        user_id: &UserId,
        channel_id: &ChannelId,
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let read_at = sqlx::query_scalar!(
            r#"
            INSERT INTO channel_read_state (user_id, channel_id, last_read_at)
            VALUES ($1, $2, now())
            ON CONFLICT (user_id, channel_id)
            DO UPDATE SET last_read_at = EXCLUDED.last_read_at
            RETURNING last_read_at
            "#,
            user_id.uuid(),
            channel_id.uuid(),
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        let recipients: Vec<Uuid> = sqlx::query_scalar!(
            r#"
            SELECT user_id
            FROM channel_text_participants
            WHERE channel_id = $1
            "#,
            channel_id.uuid(),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if !recipients.is_empty() {
            let record = read_event(
                channel_id.uuid(),
                "channel.read",
                serde_json::json!({
                    "channel_id": channel_id.uuid(),
                    "reader_id": user_id.uuid(),
                    "read_at": read_at,
                    "recipients": recipients,
                }),
                read_at,
            );
            save_outbox_records(&mut tx, std::slice::from_ref(&record)).await?;
        }

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn mark_dm_read(
        &self,
        user_id: &UserId,
        friend_id: &UserId,
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let read_at = sqlx::query_scalar!(
            r#"
            INSERT INTO dm_read_state (user_id, friend_id, last_read_at)
            VALUES ($1, $2, now())
            ON CONFLICT (user_id, friend_id)
            DO UPDATE SET last_read_at = EXCLUDED.last_read_at
            RETURNING last_read_at
            "#,
            user_id.uuid(),
            friend_id.uuid(),
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        let record = read_event(
            user_id.uuid(),
            "dm.read",
            serde_json::json!({
                "reader_id": user_id.uuid(),
                "peer_id": friend_id.uuid(),
                "read_at": read_at,
                "recipients": [friend_id.uuid(), user_id.uuid()],
            }),
            read_at,
        );
        save_outbox_records(&mut tx, std::slice::from_ref(&record)).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
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
            LEFT JOIN notification_preferences np
                ON np.user_id = $1
               AND np.scope_type = 'channel'
               AND np.scope_id = m.channel_id
            WHERE m.created_at > COALESCE(r.last_read_at, p.joined_at)
              AND m.author_id <> $1
              AND COALESCE(np.level, 'all') <> 'none'
              AND (
                  COALESCE(np.level, 'all') <> 'mentions'
                  OR $1 = ANY(m.mentions)
              )
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
            LEFT JOIN notification_preferences np
                ON np.user_id = $1
               AND np.scope_type = 'dm'
               AND np.scope_id = m.sender_id
            WHERE m.recipient_id = $1
              AND m.created_at > COALESCE(r.last_read_at, '1970-01-01T00:00:00Z'::timestamptz)
              AND COALESCE(np.level, 'all') <> 'none'
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

    async fn dm_peer_read_at(
        &self,
        user_id: &UserId,
        friend_id: &UserId,
    ) -> Result<Option<DateTime<Utc>>, RepositoryError> {
        let read_at = sqlx::query_scalar!(
            r#"
            SELECT last_read_at
            FROM dm_read_state
            WHERE user_id = $1 AND friend_id = $2
            "#,
            friend_id.uuid(),
            user_id.uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(read_at)
    }

    async fn channel_member_read_ats(
        &self,
        channel_id: &ChannelId,
        exclude: &UserId,
    ) -> Result<Vec<MemberRead>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT p.user_id, r.last_read_at AS "last_read_at?"
            FROM channel_text_participants p
            LEFT JOIN channel_read_state r
                ON r.channel_id = p.channel_id AND r.user_id = p.user_id
            WHERE p.channel_id = $1 AND p.user_id <> $2
            "#,
            channel_id.uuid(),
            exclude.uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(rows
            .into_iter()
            .map(|r| MemberRead {
                user_id: r.user_id,
                read_at: r.last_read_at,
            })
            .collect())
    }
}
