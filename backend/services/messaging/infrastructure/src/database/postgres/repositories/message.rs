use async_trait::async_trait;
use messaging_domain::message::system::SystemMessage;
use domain::{attachment::AttachmentLink, pagination::PaginatedResult, pagination::Pagination, user::id::UserId};
use ports::repository::RepositoryError;
use messaging_domain::{channel::id::ChannelId, message::Message, message::content::MessageContent, message::id::MessageId, message::repository::MessageRepository, message::view::MessageView};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::database::postgres::{
    dto::message::{MessageDto, MessageViewDto},
    error::map_sqlx_error_to_domain_error,
};

pub struct PostgresMessageRepository {
    pool: Pool<Postgres>,
}

impl PostgresMessageRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    async fn fetch_cursor(
        &self,
        channel_id: &ChannelId,
        message_id: &MessageId,
    ) -> Result<MessageCursor, RepositoryError> {
        sqlx::query!(
            r#"
            SELECT id, created_at
            FROM messages
            WHERE id = $1 AND channel_id = $2
            "#,
            message_id.uuid(),
            channel_id.uuid(),
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
impl MessageRepository for PostgresMessageRepository {
    async fn linked_attachment_subset(
        &self,
        attachment_ids: &[Uuid],
    ) -> Result<Vec<Uuid>, RepositoryError> {
        let rows = sqlx::query_scalar!(
            r#"
            SELECT attachment_id AS "attachment_id!" FROM message_attachments
            WHERE attachment_id = ANY($1)
            UNION
            SELECT attachment_id AS "attachment_id!" FROM dm_message_attachments
            WHERE attachment_id = ANY($1)
            "#,
            attachment_ids,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows)
    }

    async fn list_attachment_links_for_messages(
        &self,
        message_ids: &[uuid::Uuid],
    ) -> Result<Vec<(uuid::Uuid, AttachmentLink)>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT message_id, attachment_id, file_name, content_type, size_bytes,
                   storage_key, created_at
            FROM message_attachments
            WHERE message_id = ANY($1)
            ORDER BY created_at ASC
            "#,
            message_ids,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    r.message_id,
                    AttachmentLink {
                        id: r.attachment_id,
                        file_name: r.file_name,
                        content_type: r.content_type,
                        size_bytes: r.size_bytes,
                        storage_key: r.storage_key,
                        created_at: r.created_at,
                    },
                )
            })
            .collect())
    }

    async fn find_attachment_for_user(
        &self,
        attachment_id: uuid::Uuid,
        user_id: &UserId,
    ) -> Result<Option<AttachmentLink>, RepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT a.attachment_id AS "attachment_id!", a.file_name AS "file_name!",
                   a.content_type AS "content_type!", a.size_bytes AS "size_bytes!",
                   a.storage_key AS "storage_key!", a.created_at AS "created_at!"
            FROM message_attachments a
            INNER JOIN messages m ON m.id = a.message_id
            INNER JOIN channels c ON c.id = m.channel_id
            LEFT JOIN channel_text_participants p
                ON p.channel_id = m.channel_id AND p.user_id = $2
            LEFT JOIN server_members sm
                ON sm.server_id = c.server_id AND sm.user_id = $2
            WHERE a.attachment_id = $1
              AND (
                p.user_id IS NOT NULL
                OR (c.visibility = 'public' AND sm.user_id IS NOT NULL)
              )
            UNION ALL
            SELECT a.attachment_id AS "attachment_id!", a.file_name AS "file_name!",
                   a.content_type AS "content_type!", a.size_bytes AS "size_bytes!",
                   a.storage_key AS "storage_key!", a.created_at AS "created_at!"
            FROM dm_message_attachments a
            INNER JOIN dm_messages d ON d.id = a.dm_message_id
            WHERE a.attachment_id = $1
              AND (d.sender_id = $2 OR d.recipient_id = $2)
              AND EXISTS(
                  SELECT 1 FROM friendships f
                  WHERE f.user_id = $2
                    AND f.friend_id = CASE
                        WHEN d.sender_id = $2 THEN d.recipient_id
                        ELSE d.sender_id
                    END
                    AND f.status = 'accepted'
              )
            LIMIT 1
            "#,
            attachment_id,
            user_id.uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(row.map(|r| AttachmentLink {
            id: r.attachment_id,
            file_name: r.file_name,
            content_type: r.content_type,
            size_bytes: r.size_bytes,
            storage_key: r.storage_key,
            created_at: r.created_at,
        }))
    }

    async fn find_message_by_id(
        &self,
        message_id: &MessageId,
    ) -> Result<Message, RepositoryError> {
        let dto = sqlx::query_as!(
            MessageDto,
            r#"
            SELECT id, channel_id, author_id, content, created_at, edited_at, kind, system_payload::text AS "system_payload"
            FROM messages
            WHERE id = $1
            "#,
            message_id.uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?
        .ok_or(RepositoryError::NotFound)?;

        dto_to_message(dto)
    }

    async fn list_messages_in_channel(
        &self,
        channel_id: &ChannelId,
        before: Option<&MessageId>,
        after: Option<&MessageId>,
        limit: i64,
    ) -> Result<Vec<MessageView>, RepositoryError> {
        let dtos = match (before, after) {
            (Some(b), _) => {
                let cursor = self.fetch_cursor(channel_id, b).await?;
                sqlx::query_as!(
                    MessageViewDto,
                    r#"
                    SELECT m.id, m.channel_id, m.author_id, u.name AS author_name,
                           m.content, m.created_at, m.edited_at, m.kind, m.system_payload::text AS "system_payload"
                    FROM messages m
                    INNER JOIN messaging_user_profiles u ON u.user_id = m.author_id
                    WHERE m.channel_id = $1
                      AND (m.created_at, m.id) < ($2, $3)
                    ORDER BY m.created_at DESC, m.id DESC
                    LIMIT $4
                    "#,
                    channel_id.uuid(),
                    cursor.created_at,
                    cursor.id,
                    limit,
                )
                .fetch_all(&self.pool)
                .await
                .map_err(map_sqlx_error_to_domain_error)?
            }
            (None, Some(a)) => {
                let cursor = self.fetch_cursor(channel_id, a).await?;
                let mut rows = sqlx::query_as!(
                    MessageViewDto,
                    r#"
                    SELECT m.id, m.channel_id, m.author_id, u.name AS author_name,
                           m.content, m.created_at, m.edited_at, m.kind, m.system_payload::text AS "system_payload"
                    FROM messages m
                    INNER JOIN messaging_user_profiles u ON u.user_id = m.author_id
                    WHERE m.channel_id = $1
                      AND (m.created_at, m.id) > ($2, $3)
                    ORDER BY m.created_at ASC, m.id ASC
                    LIMIT $4
                    "#,
                    channel_id.uuid(),
                    cursor.created_at,
                    cursor.id,
                    limit,
                )
                .fetch_all(&self.pool)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
                rows.reverse();
                rows
            }
            (None, None) => sqlx::query_as!(
                MessageViewDto,
                r#"
                SELECT m.id, m.channel_id, m.author_id, u.name AS author_name,
                       m.content, m.created_at, m.edited_at, m.kind, m.system_payload::text AS "system_payload"
                FROM messages m
                INNER JOIN messaging_user_profiles u ON u.user_id = m.author_id
                WHERE m.channel_id = $1
                ORDER BY m.created_at DESC, m.id DESC
                LIMIT $2
                "#,
                channel_id.uuid(),
                limit,
            )
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error)?,
        };

        Ok(dtos.into_iter().map(view_dto_to_view).collect())
    }

    async fn search_messages_in_channel(
        &self,
        channel_id: &ChannelId,
        query: &str,
        pagination: Pagination,
    ) -> Result<PaginatedResult<MessageView>, RepositoryError> {
        let total = sqlx::query!(
            r#"
            SELECT COUNT(*) AS "count!"
            FROM messages
            WHERE channel_id = $1
              AND search_tsv @@ plainto_tsquery('simple', $2)
            "#,
            channel_id.uuid(),
            query,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?
        .count;

        let rows = sqlx::query_as!(
            MessageViewDto,
            r#"
            SELECT m.id, m.channel_id, m.author_id, u.name AS author_name,
                   m.content, m.created_at, m.edited_at, m.kind, m.system_payload::text AS "system_payload"
            FROM messages m
            INNER JOIN messaging_user_profiles u ON u.user_id = m.author_id
            WHERE m.channel_id = $1
              AND m.search_tsv @@ plainto_tsquery('simple', $2)
            ORDER BY ts_rank(m.search_tsv, plainto_tsquery('simple', $2)) DESC,
                     m.created_at DESC, m.id DESC
            LIMIT $3 OFFSET $4
            "#,
            channel_id.uuid(),
            query,
            pagination.limit(),
            pagination.offset(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        let items: Vec<MessageView> = rows.into_iter().map(view_dto_to_view).collect();
        Ok(PaginatedResult::new(items, total, pagination))
    }
}

fn dto_to_message(dto: MessageDto) -> Result<Message, RepositoryError> {
    let content = MessageContent::new_allow_empty(dto.content)
        .map_err(|e| RepositoryError::InvalidData(e.to_string()))?;
    Ok(Message::restore(
        MessageId::new(dto.id),
        ChannelId::new(dto.channel_id),
        UserId::new(dto.author_id),
        content,
        dto.created_at,
        dto.edited_at,
        system_from_row(&dto.kind, dto.system_payload.as_deref()),
    ))
}

fn system_from_row(kind: &str, payload: Option<&str>) -> Option<SystemMessage> {
    if kind != "system" {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(payload?).ok()?;
    SystemMessage::from_json(&value)
}

fn view_dto_to_view(dto: MessageViewDto) -> MessageView {
    MessageView {
        id: MessageId::new(dto.id),
        channel_id: ChannelId::new(dto.channel_id),
        author_id: UserId::new(dto.author_id),
        author_name: dto.author_name,
        content: dto.content,
        created_at: dto.created_at,
        edited_at: dto.edited_at,
        system: system_from_row(&dto.kind, dto.system_payload.as_deref()),
    }
}

pub(crate) async fn insert_message_links(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    message_id: Uuid,
    attachments: &[AttachmentLink],
) -> Result<(), RepositoryError> {
    for link in attachments {
        sqlx::query!(
            r#"
            INSERT INTO message_attachments
                (message_id, attachment_id, file_name, content_type, size_bytes, storage_key, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            message_id,
            link.id,
            link.file_name,
            link.content_type,
            link.size_bytes,
            link.storage_key,
            link.created_at,
        )
        .execute(&mut **tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
    }
    Ok(())
}
