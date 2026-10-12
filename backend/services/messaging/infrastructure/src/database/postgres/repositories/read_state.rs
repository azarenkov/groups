use async_trait::async_trait;
use chrono::{DateTime, Utc};
use messaging_domain::{channel::id::ChannelId, read_state::repository::ChannelUnread, read_state::repository::DmReadReceipt, read_state::repository::DmUnread, read_state::repository::MemberRead, read_state::repository::ReadStateRepository};
use domain::user::id::UserId;
use ports::{outbox::record::AggregateType, outbox::record::OutboxRecord, repository::RepositoryError};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::database::postgres::repositories::{friendship::block_relation_for, friendship_unit_of_work::pair_key};

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
    aggregate: (AggregateType, Uuid),
    parent: (AggregateType, Uuid),
    event_type: &str,
    payload: serde_json::Value,
    occurred_at: DateTime<Utc>,
) -> OutboxRecord {
    OutboxRecord {
        id: Uuid::new_v4(),
        aggregate_id: aggregate.1,
        aggregate_type: aggregate.0,
        parent_id: Some(parent.1),
        parent_type: Some(parent.0),
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
                (AggregateType::Message, user_id.uuid()),
                (AggregateType::Channel, channel_id.uuid()),
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
    ) -> Result<bool, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 43))")
            .bind(pair_key(user_id, friend_id))
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        if block_relation_for(&mut *tx, user_id, friend_id).await?.any() {
            return Ok(false);
        }

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
            (AggregateType::User, user_id.uuid()),
            (AggregateType::User, friend_id.uuid()),
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
        Ok(true)
    }

    async fn channel_unread_counts(
        &self,
        user_id: &UserId,
        default_level: &str,
    ) -> Result<Vec<ChannelUnread>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT m.channel_id, c.community_id, COUNT(*) AS "count!"
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
              AND NOT EXISTS (
                SELECT 1 FROM hidden_messages h
                WHERE h.user_id = $1 AND h.message_id = m.id
              )
              AND NOT EXISTS (
                SELECT 1 FROM user_blocks b
                WHERE b.blocker_id = $1 AND b.blocked_id = m.author_id
              )
              AND (
                  COALESCE(np.level, $2) <> 'mentions'
                  OR $1 = ANY(m.mentions)
              )
            GROUP BY m.channel_id, c.community_id
            "#,
            user_id.uuid(),
            default_level,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(rows
            .into_iter()
            .map(|r| ChannelUnread {
                channel_id: r.channel_id,
                community_id: r.community_id,
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
            LEFT JOIN dm_read_state r
                ON r.user_id = $1 AND r.friend_id = m.sender_id
            WHERE m.recipient_id = $1
              AND m.created_at > COALESCE(r.last_read_at, '1970-01-01T00:00:00Z'::timestamptz)
              AND NOT EXISTS (
                SELECT 1 FROM hidden_messages h
                WHERE h.user_id = $1 AND h.message_id = m.id
              )
              AND NOT EXISTS (
                SELECT 1 FROM user_blocks b
                WHERE (b.blocker_id = $1 AND b.blocked_id = m.sender_id)
                   OR (b.blocker_id = m.sender_id AND b.blocked_id = $1)
              )
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
    ) -> Result<DmReadReceipt, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 43))")
            .bind(pair_key(user_id, friend_id))
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        if block_relation_for(&mut *tx, user_id, friend_id).await?.any() {
            return Ok(DmReadReceipt::Blocked);
        }
        let read_at = sqlx::query_scalar!(
            r#"
            SELECT last_read_at
            FROM dm_read_state
            WHERE user_id = $1 AND friend_id = $2
            "#,
            friend_id.uuid(),
            user_id.uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(DmReadReceipt::ReadAt(read_at))
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

#[cfg(test)]
mod tests {
    use infrastructure_common::messaging::partitions::{conversation_key, is_conversation_partition, outbox_partition_key};

    use super::*;

    #[test]
    fn read_receipts_share_the_partition_of_their_conversation() {
        let (reader, friend, channel) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let dm = read_event(
            (AggregateType::User, reader),
            (AggregateType::User, friend),
            "dm.read",
            serde_json::json!({}),
            Utc::now(),
        );
        assert!(is_conversation_partition(&dm));
        assert_eq!(outbox_partition_key(&dm), conversation_key(reader, friend));

        let channel_read = read_event(
            (AggregateType::Message, reader),
            (AggregateType::Channel, channel),
            "channel.read",
            serde_json::json!({}),
            Utc::now(),
        );
        assert!(is_conversation_partition(&channel_read));
        assert_eq!(outbox_partition_key(&channel_read), channel);
    }
}
