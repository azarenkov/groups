use async_trait::async_trait;
use domain::{
    dm_message::{
        DmMessage, id::DmMessageId,
        repository::{DmConversationPreview, DmMessageRepository},
        view::DmMessageView,
    },
    message::content::MessageContent,
    server::id::ServerId,

    shared::{
        outbox::outbox_record::OutboxRecord,
        pagination::{PaginatedResult, Pagination},
        repository::error::RepositoryError,
    },
    user::id::UserId,
};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::database::postgres::{
    error::map_sqlx_error_to_domain_error,
    repositories::outbox_record::save_outbox_records,
};

pub struct PostgresDmMessageRepository {
    pool: Pool<Postgres>,
}

impl PostgresDmMessageRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    async fn fetch_cursor(
        &self,
        user_a: &UserId,
        user_b: &UserId,
        message_id: &DmMessageId,
    ) -> Result<MessageCursor, RepositoryError> {
        sqlx::query!(
            r#"
            SELECT id, created_at
            FROM dm_messages
            WHERE id = $1
              AND LEAST(sender_id, recipient_id) = LEAST($2::uuid, $3::uuid)
              AND GREATEST(sender_id, recipient_id) = GREATEST($2::uuid, $3::uuid)
            "#,
            message_id.uuid(),
            user_a.uuid(),
            user_b.uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?
        .map(|r| MessageCursor {
            id: r.id,
            created_at: r.created_at,
        })
        .ok_or(RepositoryError::NotFound)
    }
}

struct MessageCursor {
    id: uuid::Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
impl DmMessageRepository for PostgresDmMessageRepository {
    async fn save_with_events(
        &self,
        message: &DmMessage,
        events: &[OutboxRecord],
        attachment_ids: &[Uuid],
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        sqlx::query!(
            r#"
            INSERT INTO dm_messages (id, sender_id, recipient_id, content, invite_server_id, call_status, call_duration_seconds, created_at, edited_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            message.id().uuid(),
            message.sender_id().uuid(),
            message.recipient_id().uuid(),
            message.content().as_str(),
            message.invite_server_id().map(|id| id.uuid()),
            message.call_status(),
            message.call_duration_seconds(),
            message.created_at(),
            message.edited_at(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if !attachment_ids.is_empty() {
            sqlx::query!(
                r#"
                UPDATE attachments
                SET dm_message_id = $3
                WHERE id = ANY($1) AND owner_id = $2
                  AND message_id IS NULL AND dm_message_id IS NULL
                "#,
                attachment_ids,
                message.sender_id().uuid(),
                message.id().uuid(),
            )
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        }
        save_outbox_records(&mut tx, events).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn update_with_events(
        &self,
        message: &DmMessage,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        let result = sqlx::query!(
            r#"
            UPDATE dm_messages
            SET content = $2, edited_at = $3
            WHERE id = $1
            "#,
            message.id().uuid(),
            message.content().as_str(),
            message.edited_at(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        save_outbox_records(&mut tx, events).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn delete_with_events(
        &self,
        id: &DmMessageId,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        let result = sqlx::query!(
            r#"
            DELETE FROM dm_messages WHERE id = $1
            "#,
            id.uuid(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        save_outbox_records(&mut tx, events).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn find_by_id(&self, id: &DmMessageId) -> Result<DmMessage, RepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT id, sender_id, recipient_id, content, invite_server_id, call_status, call_duration_seconds, created_at, edited_at
            FROM dm_messages
            WHERE id = $1
            "#,
            id.uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?
        .ok_or(RepositoryError::NotFound)?;

        let content = MessageContent::new(row.content)
            .map_err(|e| RepositoryError::InvalidData(e.to_string()))?;

        Ok(DmMessage::restore(
            DmMessageId::new(row.id),
            UserId::new(row.sender_id),
            UserId::new(row.recipient_id),
            content,
            row.invite_server_id.map(ServerId::new),
            row.call_status,
            row.call_duration_seconds,
            row.created_at,
            row.edited_at,
        ))
    }

    async fn list_between(
        &self,
        user_a: &UserId,
        user_b: &UserId,
        before: Option<&DmMessageId>,
        after: Option<&DmMessageId>,
        limit: i64,
    ) -> Result<Vec<DmMessageView>, RepositoryError> {
        let rows: Vec<DmMessageView> = match (before, after) {
            (Some(b), _) => {
                let cursor = self.fetch_cursor(user_a, user_b, b).await?;
                sqlx::query!(
                    r#"
                    SELECT m.id, m.sender_id, m.recipient_id,
                           u.name AS sender_name,
                           m.content, m.invite_server_id,
                           s.name AS "invite_server_name?",
                           m.call_status, m.call_duration_seconds,
                           m.created_at, m.edited_at
                    FROM dm_messages m
                    INNER JOIN users u ON u.id = m.sender_id
                    LEFT JOIN servers s ON s.id = m.invite_server_id
                    WHERE LEAST(m.sender_id, m.recipient_id) = LEAST($1::uuid, $2::uuid)
                      AND GREATEST(m.sender_id, m.recipient_id) = GREATEST($1::uuid, $2::uuid)
                      AND (m.created_at, m.id) < ($3, $4)
                    ORDER BY m.created_at DESC, m.id DESC
                    LIMIT $5
                    "#,
                    user_a.uuid(),
                    user_b.uuid(),
                    cursor.created_at,
                    cursor.id,
                    limit,
                )
                .fetch_all(&self.pool)
                .await
                .map_err(map_sqlx_error_to_domain_error)?
                .into_iter()
                .map(|r| DmMessageView {
                    id: DmMessageId::new(r.id),
                    sender_id: UserId::new(r.sender_id),
                    sender_name: r.sender_name,
                    recipient_id: UserId::new(r.recipient_id),
                    content: r.content,
                    invite_server_id: r.invite_server_id.map(ServerId::new),
                    invite_server_name: r.invite_server_name,
                    call_status: r.call_status,
                    call_duration_seconds: r.call_duration_seconds,
                    created_at: r.created_at,
                    edited_at: r.edited_at,
                })
                .collect()
            }
            (None, Some(a)) => {
                let cursor = self.fetch_cursor(user_a, user_b, a).await?;
                let mut rows: Vec<DmMessageView> = sqlx::query!(
                    r#"
                    SELECT m.id, m.sender_id, m.recipient_id,
                           u.name AS sender_name,
                           m.content, m.invite_server_id,
                           s.name AS "invite_server_name?",
                           m.call_status, m.call_duration_seconds,
                           m.created_at, m.edited_at
                    FROM dm_messages m
                    INNER JOIN users u ON u.id = m.sender_id
                    LEFT JOIN servers s ON s.id = m.invite_server_id
                    WHERE LEAST(m.sender_id, m.recipient_id) = LEAST($1::uuid, $2::uuid)
                      AND GREATEST(m.sender_id, m.recipient_id) = GREATEST($1::uuid, $2::uuid)
                      AND (m.created_at, m.id) > ($3, $4)
                    ORDER BY m.created_at ASC, m.id ASC
                    LIMIT $5
                    "#,
                    user_a.uuid(),
                    user_b.uuid(),
                    cursor.created_at,
                    cursor.id,
                    limit,
                )
                .fetch_all(&self.pool)
                .await
                .map_err(map_sqlx_error_to_domain_error)?
                .into_iter()
                .map(|r| DmMessageView {
                    id: DmMessageId::new(r.id),
                    sender_id: UserId::new(r.sender_id),
                    sender_name: r.sender_name,
                    recipient_id: UserId::new(r.recipient_id),
                    content: r.content,
                    invite_server_id: r.invite_server_id.map(ServerId::new),
                    invite_server_name: r.invite_server_name,
                    call_status: r.call_status,
                    call_duration_seconds: r.call_duration_seconds,
                    created_at: r.created_at,
                    edited_at: r.edited_at,
                })
                .collect();
                rows.reverse();
                rows
            }
            (None, None) => sqlx::query!(
                r#"
                SELECT m.id, m.sender_id, m.recipient_id,
                       u.name AS sender_name,
                       m.content, m.invite_server_id,
                       s.name AS "invite_server_name?",
                       m.call_status, m.call_duration_seconds,
                       m.created_at, m.edited_at
                FROM dm_messages m
                INNER JOIN users u ON u.id = m.sender_id
                LEFT JOIN servers s ON s.id = m.invite_server_id
                WHERE LEAST(m.sender_id, m.recipient_id) = LEAST($1::uuid, $2::uuid)
                  AND GREATEST(m.sender_id, m.recipient_id) = GREATEST($1::uuid, $2::uuid)
                ORDER BY m.created_at DESC, m.id DESC
                LIMIT $3
                "#,
                user_a.uuid(),
                user_b.uuid(),
                limit,
            )
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error)?
            .into_iter()
            .map(|r| DmMessageView {
                id: DmMessageId::new(r.id),
                sender_id: UserId::new(r.sender_id),
                sender_name: r.sender_name,
                recipient_id: UserId::new(r.recipient_id),
                content: r.content,
                invite_server_id: r.invite_server_id.map(ServerId::new),
                invite_server_name: r.invite_server_name,
                call_status: r.call_status,
                call_duration_seconds: r.call_duration_seconds,
                created_at: r.created_at,
                edited_at: r.edited_at,
            })
            .collect(),
        };

        Ok(rows)
    }

    async fn search_between(
        &self,
        user_a: &UserId,
        user_b: &UserId,
        query: &str,
        pagination: Pagination,
    ) -> Result<PaginatedResult<DmMessageView>, RepositoryError> {
        let total = sqlx::query!(
            r#"
            SELECT COUNT(*) AS "count!"
            FROM dm_messages
            WHERE LEAST(sender_id, recipient_id) = LEAST($1::uuid, $2::uuid)
              AND GREATEST(sender_id, recipient_id) = GREATEST($1::uuid, $2::uuid)
              AND search_tsv @@ plainto_tsquery('simple', $3)
            "#,
            user_a.uuid(),
            user_b.uuid(),
            query,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?
        .count;

        let rows = sqlx::query!(
            r#"
            SELECT m.id, m.sender_id, m.recipient_id,
                   u.name AS sender_name,
                   m.content, m.invite_server_id,
                   s.name AS "invite_server_name?",
                   m.call_status, m.call_duration_seconds,
                   m.created_at, m.edited_at
            FROM dm_messages m
            INNER JOIN users u ON u.id = m.sender_id
            LEFT JOIN servers s ON s.id = m.invite_server_id
            WHERE LEAST(m.sender_id, m.recipient_id) = LEAST($1::uuid, $2::uuid)
              AND GREATEST(m.sender_id, m.recipient_id) = GREATEST($1::uuid, $2::uuid)
              AND m.search_tsv @@ plainto_tsquery('simple', $3)
            ORDER BY ts_rank(m.search_tsv, plainto_tsquery('simple', $3)) DESC,
                     m.created_at DESC, m.id DESC
            LIMIT $4 OFFSET $5
            "#,
            user_a.uuid(),
            user_b.uuid(),
            query,
            pagination.limit(),
            pagination.offset(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        let items: Vec<DmMessageView> = rows
            .into_iter()
            .map(|r| DmMessageView {
                id: DmMessageId::new(r.id),
                sender_id: UserId::new(r.sender_id),
                sender_name: r.sender_name,
                recipient_id: UserId::new(r.recipient_id),
                content: r.content,
                invite_server_id: r.invite_server_id.map(ServerId::new),
                invite_server_name: r.invite_server_name,
                call_status: r.call_status,
                call_duration_seconds: r.call_duration_seconds,
                created_at: r.created_at,
                edited_at: r.edited_at,
            })
            .collect();
        Ok(PaginatedResult::new(items, total, pagination))
    }

    async fn has_pending_server_invite(
        &self,
        sender_id: &UserId,
        recipient_id: &UserId,
        server_id: &ServerId,
    ) -> Result<bool, RepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM dm_messages
                WHERE sender_id = $1
                  AND recipient_id = $2
                  AND invite_server_id = $3
            ) AS "exists!"
            "#,
            sender_id.uuid(),
            recipient_id.uuid(),
            server_id.uuid(),
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.exists)
    }

    async fn find_invite_recipients(
        &self,
        sender_id: &UserId,
        server_id: &ServerId,
    ) -> Result<Vec<UserId>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT DISTINCT recipient_id
            FROM dm_messages
            WHERE sender_id = $1 AND invite_server_id = $2
            "#,
            sender_id.uuid(),
            server_id.uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows
            .into_iter()
            .map(|r| UserId::new(r.recipient_id))
            .collect())
    }

    async fn list_conversation_previews(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<DmConversationPreview>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT DISTINCT ON (m.peer)
                m.id AS "message_id!",
                m.peer AS "peer!",
                m.sender_id AS "sender_id!",
                m.content AS "content!",
                m.call_status,
                m.invite_server_id,
                m.created_at AS "created_at!"
            FROM (
                SELECT
                    id,
                    CASE WHEN sender_id = $1 THEN recipient_id ELSE sender_id END AS peer,
                    sender_id,
                    content,
                    call_status,
                    invite_server_id,
                    created_at
                FROM dm_messages
                WHERE sender_id = $1 OR recipient_id = $1
            ) m
            ORDER BY m.peer, m.created_at DESC
            "#,
            user_id.uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(rows
            .into_iter()
            .map(|r| DmConversationPreview {
                message_id: r.message_id,
                peer_id: r.peer,
                sender_id: r.sender_id,
                content: r.content,
                call_status: r.call_status,
                invite_server_id: r.invite_server_id,
                created_at: r.created_at,
            })
            .collect())
    }
}
