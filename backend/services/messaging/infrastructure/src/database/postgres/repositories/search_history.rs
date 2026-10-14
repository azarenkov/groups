use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use messaging_domain::search::{SearchPlace, snippet};
use messaging_domain::search_history::{
    HistoryEntry, HistoryItem, HistoryTarget, RecordedEntry, SEARCH_HISTORY_LIMIT, SearchHistoryRepository,
};
use ports::{outbox::record::AggregateType, outbox::record::OutboxRecord, repository::RepositoryError};
use sqlx::{Pool, Postgres, Transaction};
use uuid::Uuid;

use crate::database::postgres::{
    error::map_sqlx_error_to_domain_error,
    repositories::{community_unit_of_work::lock_user, outbox_record::save_outbox_records},
};

pub struct PostgresSearchHistoryRepository {
    pool: Pool<Postgres>,
}

impl PostgresSearchHistoryRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    async fn people(&self, viewer: Uuid, ids: &[Uuid]) -> Result<HashMap<Uuid, String>, RepositoryError> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows = sqlx::query!(
            r#"
            SELECT p.user_id, p.name FROM messaging_user_profiles p
            WHERE p.user_id = ANY($2)
              AND p.active
              AND NOT EXISTS (SELECT 1 FROM messaging_deleted_users d WHERE d.user_id = p.user_id)
              AND NOT EXISTS (
                  SELECT 1 FROM user_blocks b
                  WHERE (b.blocker_id = $1 AND b.blocked_id = p.user_id)
                     OR (b.blocker_id = p.user_id AND b.blocked_id = $1)
              )
            "#,
            viewer,
            ids,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows.into_iter().map(|r| (r.user_id, r.name)).collect())
    }

    async fn communities(&self, viewer: Uuid, ids: &[Uuid]) -> Result<HashMap<Uuid, String>, RepositoryError> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows = sqlx::query!(
            r#"
            SELECT s.id, s.name FROM communities s
            WHERE s.id = ANY($2)
              AND EXISTS (SELECT 1 FROM community_members m WHERE m.community_id = s.id AND m.user_id = $1)
            "#,
            viewer,
            ids,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows.into_iter().map(|r| (r.id, r.name)).collect())
    }

    async fn channels(&self, viewer: Uuid, ids: &[Uuid]) -> Result<HashMap<Uuid, HistoryTarget>, RepositoryError> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows = sqlx::query!(
            r#"
            SELECT c.id, c.name, c.kind = 'voice' AS "voice!", s.id AS community_id, s.name AS community_name
            FROM channels c
            JOIN communities s ON s.id = c.community_id
            WHERE c.id = ANY($2)
              AND EXISTS (SELECT 1 FROM community_members m WHERE m.community_id = s.id AND m.user_id = $1)
            "#,
            viewer,
            ids,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    r.id,
                    HistoryTarget::Channel {
                        community_id: r.community_id,
                        community_name: r.community_name,
                        channel_id: r.id,
                        name: r.name,
                        voice: r.voice,
                    },
                )
            })
            .collect())
    }

    async fn messages(&self, viewer: Uuid, ids: &[Uuid]) -> Result<HashMap<Uuid, HistoryTarget>, RepositoryError> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let channel_rows = sqlx::query!(
            r#"
            SELECT m.id, m.author_id, COALESCE(u.name, '') AS "author_name!", m.created_at, m.content,
                   (m.forwarded_from->>'authorId')::uuid AS "forwarded_author_id?",
                   c.id AS channel_id, c.name AS channel_name, s.id AS community_id, s.name AS community_name
            FROM messages m
            JOIN channels c ON c.id = m.channel_id AND c.kind = 'text'
            JOIN communities s ON s.id = c.community_id
            LEFT JOIN messaging_user_profiles u ON u.user_id = m.author_id
            WHERE m.id = ANY($2)
              AND m.kind = 'user'
              AND EXISTS (SELECT 1 FROM community_members cm WHERE cm.community_id = c.community_id AND cm.user_id = $1)
              AND (c.visibility = 'public'
                   OR EXISTS (SELECT 1 FROM channel_text_participants p WHERE p.channel_id = c.id AND p.user_id = $1))
              AND NOT EXISTS (SELECT 1 FROM hidden_messages h WHERE h.user_id = $1 AND h.message_id = m.id)
            "#,
            viewer,
            ids,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let dm_rows = sqlx::query!(
            r#"
            SELECT m.id, m.sender_id, COALESCE(u.name, '') AS "author_name!", m.created_at, m.content,
                   (m.forwarded_from->>'authorId')::uuid AS "forwarded_author_id?",
                   peer.id AS "peer_id!", COALESCE(pu.name, '') AS "peer_name!"
            FROM dm_messages m
            CROSS JOIN LATERAL (SELECT CASE WHEN m.sender_id = $1 THEN m.recipient_id ELSE m.sender_id END AS id) peer
            LEFT JOIN messaging_user_profiles u ON u.user_id = m.sender_id
            LEFT JOIN messaging_user_profiles pu ON pu.user_id = peer.id
            WHERE m.id = ANY($2)
              AND (m.sender_id = $1 OR m.recipient_id = $1)
              AND m.call_status IS NULL
              AND NOT EXISTS (
                  SELECT 1 FROM user_blocks b
                  WHERE (b.blocker_id = $1 AND b.blocked_id = peer.id)
                     OR (b.blocker_id = peer.id AND b.blocked_id = $1)
              )
              AND NOT EXISTS (SELECT 1 FROM hidden_messages h WHERE h.user_id = $1 AND h.message_id = m.id)
            "#,
            viewer,
            ids,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let channel = channel_rows.into_iter().map(|r| {
            (
                r.id,
                HistoryTarget::Message {
                    message_id: r.id,
                    author_id: r.author_id,
                    author_name: r.author_name,
                    forwarded_author_id: r.forwarded_author_id,
                    snippet: snippet(&r.content, ""),
                    created_at: r.created_at,
                    place: SearchPlace::Channel {
                        community_id: r.community_id,
                        community_name: r.community_name,
                        channel_id: r.channel_id,
                        channel_name: r.channel_name,
                    },
                },
            )
        });
        let dm = dm_rows.into_iter().map(|r| {
            (
                r.id,
                HistoryTarget::Message {
                    message_id: r.id,
                    author_id: r.sender_id,
                    author_name: r.author_name,
                    forwarded_author_id: r.forwarded_author_id,
                    snippet: snippet(&r.content, ""),
                    created_at: r.created_at,
                    place: SearchPlace::Dm { peer_id: r.peer_id, peer_name: r.peer_name },
                },
            )
        });
        Ok(channel.chain(dm).collect())
    }
}

struct StoredEntry {
    id: Uuid,
    kind: String,
    query: Option<String>,
    target_id: Option<Uuid>,
    used_at: DateTime<Utc>,
}

fn ids_of<'a>(entries: &'a [StoredEntry], kinds: &'a [&str]) -> Vec<Uuid> {
    entries
        .iter()
        .filter(|e| kinds.contains(&e.kind.as_str()))
        .filter_map(|e| e.target_id)
        .collect()
}

#[async_trait]
impl SearchHistoryRepository for PostgresSearchHistoryRepository {
    async fn list(&self, user: &UserId) -> Result<Vec<HistoryItem>, RepositoryError> {
        let viewer = user.uuid();
        let entries: Vec<StoredEntry> = sqlx::query_as!(
            StoredEntry,
            r#"
            SELECT id, kind, query, target_id, used_at FROM search_history
            WHERE user_id = $1
            ORDER BY used_at DESC, id
            LIMIT $2
            "#,
            viewer,
            SEARCH_HISTORY_LIMIT,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let people = self.people(viewer, &ids_of(&entries, &["person", "dm"])).await?;
        let communities = self.communities(viewer, &ids_of(&entries, &["community"])).await?;
        let channels = self.channels(viewer, &ids_of(&entries, &["channel"])).await?;
        let messages = self.messages(viewer, &ids_of(&entries, &["message"])).await?;
        Ok(entries
            .into_iter()
            .filter_map(|e| {
                let target = match (e.kind.as_str(), e.query, e.target_id) {
                    ("query", Some(text), _) => Some(HistoryTarget::Query { text }),
                    ("person", _, Some(id)) => people.get(&id).map(|name| HistoryTarget::Person { user_id: id, name: name.clone() }),
                    ("dm", _, Some(id)) => people.get(&id).map(|name| HistoryTarget::Dm { user_id: id, name: name.clone() }),
                    ("community", _, Some(id)) => communities
                        .get(&id)
                        .map(|name| HistoryTarget::Community { community_id: id, name: name.clone() }),
                    ("channel", _, Some(id)) => channels.get(&id).cloned(),
                    ("message", _, Some(id)) => messages.get(&id).cloned(),
                    _ => None,
                }?;
                Some(HistoryItem { id: e.id, used_at: e.used_at, target })
            })
            .collect())
    }

    async fn record(&self, user: &UserId, entry: &HistoryEntry, at: DateTime<Utc>) -> Result<RecordedEntry, RepositoryError> {
        let mut tx = begin_history_tx(&self.pool, user).await?;
        let row = sqlx::query!(
            r#"
            INSERT INTO search_history (id, user_id, kind, key, query, target_id, used_at)
            SELECT $1, $2, $3, $4, $5, $6, $7
            WHERE NOT EXISTS (SELECT 1 FROM messaging_deleted_users d WHERE d.user_id = $2)
            ON CONFLICT (user_id, kind, key) DO UPDATE
                SET used_at = GREATEST(search_history.used_at, EXCLUDED.used_at),
                    query = CASE
                        WHEN EXCLUDED.used_at >= search_history.used_at THEN EXCLUDED.query
                        ELSE search_history.query
                    END
            RETURNING id, used_at
            "#,
            Uuid::new_v4(),
            user.uuid(),
            entry.kind(),
            entry.key(),
            entry.query(),
            entry.target(),
            at,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?
        .ok_or(RepositoryError::NotFound)?;
        sqlx::query!(
            r#"
            DELETE FROM search_history
            WHERE user_id = $1
              AND id NOT IN (
                  SELECT id FROM search_history WHERE user_id = $1 ORDER BY used_at DESC, id LIMIT $2
              )
            "#,
            user.uuid(),
            SEARCH_HISTORY_LIMIT,
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        save_history_event(&mut tx, user).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(RecordedEntry { id: row.id, used_at: row.used_at })
    }

    async fn remove(&self, user: &UserId, id: Uuid) -> Result<(), RepositoryError> {
        let mut tx = begin_history_tx(&self.pool, user).await?;
        let removed = sqlx::query!("DELETE FROM search_history WHERE user_id = $1 AND id = $2", user.uuid(), id)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?
            .rows_affected();
        if removed > 0 {
            save_history_event(&mut tx, user).await?;
        }
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)
    }

    async fn clear(&self, user: &UserId) -> Result<(), RepositoryError> {
        let mut tx = begin_history_tx(&self.pool, user).await?;
        let removed = sqlx::query!("DELETE FROM search_history WHERE user_id = $1", user.uuid())
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?
            .rows_affected();
        if removed > 0 {
            save_history_event(&mut tx, user).await?;
        }
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)
    }
}

async fn begin_history_tx<'a>(pool: &'a Pool<Postgres>, user: &UserId) -> Result<Transaction<'a, Postgres>, RepositoryError> {
    let mut tx = pool.begin().await.map_err(map_sqlx_error_to_domain_error)?;
    lock_user(&mut tx, user).await?;
    Ok(tx)
}

async fn save_history_event(tx: &mut Transaction<'_, Postgres>, user: &UserId) -> Result<(), RepositoryError> {
    let record = OutboxRecord {
        id: Uuid::new_v4(),
        aggregate_id: user.uuid(),
        aggregate_type: AggregateType::User,
        parent_id: None,
        parent_type: None,
        event_type: "search_history.updated".to_string(),
        payload: serde_json::json!({ "user_id": user.uuid() }),
        occurred_at: Utc::now(),
        request_id: None,
    };
    save_outbox_records(tx, std::slice::from_ref(&record)).await
}
