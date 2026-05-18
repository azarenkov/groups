use async_trait::async_trait;
use domain::{
    dm_message::{
        DmMessage, id::DmMessageId, repository::DmMessageRepository, view::DmMessageView,
    },
    message::content::MessageContent,
    shared::{
        pagination::{PaginatedResult, Pagination},
        repository::error::RepositoryError,
    },
    user::id::UserId,
};
use sqlx::{Pool, Postgres};

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

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
              AND ((sender_id = $2 AND recipient_id = $3)
                OR (sender_id = $3 AND recipient_id = $2))
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
    async fn save(&self, message: &DmMessage) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            INSERT INTO dm_messages (id, sender_id, recipient_id, content, created_at, edited_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            message.id().uuid(),
            message.sender_id().uuid(),
            message.recipient_id().uuid(),
            message.content().as_str(),
            message.created_at(),
            message.edited_at(),
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn update(&self, message: &DmMessage) -> Result<(), RepositoryError> {
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
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        Ok(())
    }

    async fn delete(&self, id: &DmMessageId) -> Result<(), RepositoryError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM dm_messages WHERE id = $1
            "#,
            id.uuid(),
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        Ok(())
    }

    async fn find_by_id(&self, id: &DmMessageId) -> Result<DmMessage, RepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT id, sender_id, recipient_id, content, created_at, edited_at
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
                           m.content, m.created_at, m.edited_at
                    FROM dm_messages m
                    INNER JOIN users u ON u.id = m.sender_id
                    WHERE ((m.sender_id = $1 AND m.recipient_id = $2)
                       OR (m.sender_id = $2 AND m.recipient_id = $1))
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
                           m.content, m.created_at, m.edited_at
                    FROM dm_messages m
                    INNER JOIN users u ON u.id = m.sender_id
                    WHERE ((m.sender_id = $1 AND m.recipient_id = $2)
                       OR (m.sender_id = $2 AND m.recipient_id = $1))
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
                       m.content, m.created_at, m.edited_at
                FROM dm_messages m
                INNER JOIN users u ON u.id = m.sender_id
                WHERE (m.sender_id = $1 AND m.recipient_id = $2)
                   OR (m.sender_id = $2 AND m.recipient_id = $1)
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
            WHERE ((sender_id = $1 AND recipient_id = $2)
               OR (sender_id = $2 AND recipient_id = $1))
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
                   m.content, m.created_at, m.edited_at
            FROM dm_messages m
            INNER JOIN users u ON u.id = m.sender_id
            WHERE ((m.sender_id = $1 AND m.recipient_id = $2)
               OR (m.sender_id = $2 AND m.recipient_id = $1))
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
                created_at: r.created_at,
                edited_at: r.edited_at,
            })
            .collect();
        Ok(PaginatedResult::new(items, total, pagination))
    }
}
