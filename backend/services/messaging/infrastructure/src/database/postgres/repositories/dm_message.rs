use chrono::{DateTime, Utc};
use domain::pagination::KeysetCursor;
use async_trait::async_trait;
use domain::{attachment::AttachmentLink, attachment::VideoNoteMeta, attachment::VoiceMeta, pagination::PaginatedResult, pagination::Pagination, user::id::UserId};
use ports::repository::RepositoryError;
use messaging_domain::{dm_message::DmMessage, dm_message::id::DmMessageId, dm_message::repository::DmConversationPreview, dm_message::repository::DmMessageRepository, dm_message::view::DmMessageView, message::content::MessageContent, message::view::ReplyPreviewView, community::id::CommunityId};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

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

pub(crate) async fn insert_dm_message(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    message: &DmMessage,
) -> Result<(), RepositoryError> {
    sqlx::query!(
        r#"
        INSERT INTO dm_messages (id, sender_id, recipient_id, content, invite_community_id, call_status, call_duration_seconds, call_id, created_at, edited_at, reply_to_message_id, forwarded_from, sticker)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, ($12::text)::jsonb, ($13::text)::jsonb)
        "#,
        message.id().uuid(),
        message.sender_id().uuid(),
        message.recipient_id().uuid(),
        message.content().as_str(),
        message.invite_community_id().map(|id| id.uuid()),
        message.call_status(),
        message.call_duration_seconds(),
        message.call_id(),
        message.created_at(),
        message.edited_at(),
        message.reply().map(|r| r.message_id),
        message.forward().map(|f| f.to_json().to_string()),
        message.sticker().map(|s| s.to_json().to_string()),
    )
    .execute(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?;
    Ok(())
}

#[async_trait]
impl DmMessageRepository for PostgresDmMessageRepository {
    async fn linked_attachment_subset(
        &self,
        attachment_ids: &[uuid::Uuid],
    ) -> Result<Vec<uuid::Uuid>, RepositoryError> {
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

    async fn list_users_hiding(&self, message_id: Uuid) -> Result<Vec<UserId>, RepositoryError> {
        let rows = sqlx::query_scalar!(
            r#"SELECT user_id FROM hidden_messages WHERE message_id = $1"#,
            message_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows.into_iter().map(UserId::new).collect())
    }

    async fn list_attachment_links_for_dm_messages(
        &self,
        dm_message_ids: &[Uuid],
    ) -> Result<Vec<(Uuid, AttachmentLink)>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT dm_message_id, attachment_id, file_name, content_type, size_bytes,
                   storage_key, created_at, voice_duration_ms, voice_waveform,
                   video_duration_ms, video_size, video_poster
            FROM dm_message_attachments
            WHERE dm_message_id = ANY($1)
            ORDER BY created_at ASC
            "#,
            dm_message_ids,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    r.dm_message_id,
                    AttachmentLink {
                        id: r.attachment_id,
                        file_name: r.file_name,
                        content_type: r.content_type,
                        size_bytes: r.size_bytes,
                        storage_key: r.storage_key,
                        created_at: r.created_at,
                        voice: VoiceMeta::from_columns(r.voice_duration_ms, r.voice_waveform),
                        video_note: VideoNoteMeta::from_columns(r.video_duration_ms, r.video_size, r.video_poster),
                    },
                )
            })
            .collect())
    }

    async fn find_by_id(&self, id: &DmMessageId) -> Result<DmMessage, RepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT id, sender_id, recipient_id, content, invite_community_id, call_status, call_duration_seconds, created_at, edited_at, forwarded_from::text AS "forwarded_from?", sticker::text AS "sticker?"
            FROM dm_messages
            WHERE id = $1
            "#,
            id.uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?
        .ok_or(RepositoryError::NotFound)?;

        let content = MessageContent::new_allow_empty(row.content)
            .map_err(|e| RepositoryError::InvalidData(e.to_string()))?;

        let message = DmMessage::restore(
            DmMessageId::new(row.id),
            UserId::new(row.sender_id),
            UserId::new(row.recipient_id),
            content,
            row.invite_community_id.map(CommunityId::new),
            row.call_status,
            row.call_duration_seconds,
            row.created_at,
            row.edited_at,
        );
        let message = match parse_forward(row.forwarded_from.as_deref()) {
            Some(forward) => message.with_forward(forward),
            None => message,
        };
        Ok(message.with_sticker(parse_sticker(row.sticker.as_deref())))
    }

    async fn find_view_between(
        &self,
        id: &DmMessageId,
        user_a: &UserId,
        user_b: &UserId,
    ) -> Result<DmMessageView, RepositoryError> {
        let r = sqlx::query!(
            r#"
            SELECT m.id, m.sender_id, m.recipient_id,
                   u.name AS sender_name,
                   m.content, m.invite_community_id,
                   s.name AS "invite_community_name?",
                   m.call_status, m.call_duration_seconds,
                   m.created_at, m.edited_at, m.reply_to_message_id, m.forwarded_from::text AS "forwarded_from?", m.sticker::text AS "sticker?"
            FROM dm_messages m
            INNER JOIN messaging_user_profiles u ON u.user_id = m.sender_id
            LEFT JOIN communities s ON s.id = m.invite_community_id
            WHERE m.id = $1
              AND LEAST(m.sender_id, m.recipient_id) = LEAST($2::uuid, $3::uuid)
              AND GREATEST(m.sender_id, m.recipient_id) = GREATEST($2::uuid, $3::uuid)
              AND NOT EXISTS (SELECT 1 FROM hidden_messages h WHERE h.user_id = $2 AND h.message_id = m.id)
            "#,
            id.uuid(),
            user_a.uuid(),
            user_b.uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?
        .ok_or(RepositoryError::NotFound)?;
        Ok(DmMessageView {
            id: DmMessageId::new(r.id),
            sender_id: UserId::new(r.sender_id),
            sender_name: r.sender_name,
            recipient_id: UserId::new(r.recipient_id),
            content: r.content,
            invite_community_id: r.invite_community_id.map(CommunityId::new),
            invite_community_name: r.invite_community_name,
            call_status: r.call_status,
            call_duration_seconds: r.call_duration_seconds,
            created_at: r.created_at,
            edited_at: r.edited_at,
            reply_to: r.reply_to_message_id,
            forwarded_from: parse_forward(r.forwarded_from.as_deref()),
                sticker: parse_sticker(r.sticker.as_deref()),
        })
    }

    async fn list_dm_reply_previews(
        &self,
        dm_message_ids: &[Uuid],
        viewer: &UserId,
    ) -> Result<Vec<ReplyPreviewView>, RepositoryError> {
        if dm_message_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query!(
            r#"
            SELECT m.id, m.sender_id, u.name AS sender_name, m.content,
                   EXISTS(SELECT 1 FROM dm_message_attachments a WHERE a.dm_message_id = m.id) AS "has_attachments!",
                   (SELECT a.voice_duration_ms FROM dm_message_attachments a
                    WHERE a.dm_message_id = m.id AND a.voice_duration_ms IS NOT NULL LIMIT 1) AS voice_duration_ms,
                   (SELECT a.video_duration_ms FROM dm_message_attachments a
                    WHERE a.dm_message_id = m.id AND a.video_duration_ms IS NOT NULL LIMIT 1) AS video_note_duration_ms,
                   (SELECT a.video_poster FROM dm_message_attachments a
                    WHERE a.dm_message_id = m.id AND a.video_poster IS NOT NULL LIMIT 1) AS video_note_poster,
                   m.sticker::text AS "sticker?"
            FROM dm_messages m
            INNER JOIN messaging_user_profiles u ON u.user_id = m.sender_id
            WHERE m.id = ANY($1)
              AND NOT EXISTS (SELECT 1 FROM hidden_messages h WHERE h.user_id = $2 AND h.message_id = m.id)
            "#,
            dm_message_ids,
            viewer.uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows
            .into_iter()
            .map(|r| ReplyPreviewView {
                message_id: r.id,
                author_id: UserId::new(r.sender_id),
                author_name: r.sender_name,
                content: r.content,
                has_attachments: r.has_attachments,
                voice_duration_ms: r.voice_duration_ms,
                video_note_duration_ms: r.video_note_duration_ms,
                video_note_poster: r.video_note_poster,
                sticker: parse_sticker(r.sticker.as_deref()),
            })
            .collect())
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
                           m.content, m.invite_community_id,
                           s.name AS "invite_community_name?",
                           m.call_status, m.call_duration_seconds,
                           m.created_at, m.edited_at, m.reply_to_message_id, m.forwarded_from::text AS "forwarded_from?", m.sticker::text AS "sticker?"
                    FROM dm_messages m
                    INNER JOIN messaging_user_profiles u ON u.user_id = m.sender_id
                    LEFT JOIN communities s ON s.id = m.invite_community_id
                    WHERE LEAST(m.sender_id, m.recipient_id) = LEAST($1::uuid, $2::uuid)
                      AND GREATEST(m.sender_id, m.recipient_id) = GREATEST($1::uuid, $2::uuid)
                      AND NOT EXISTS (SELECT 1 FROM hidden_messages h WHERE h.user_id = $1 AND h.message_id = m.id)
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
                    invite_community_id: r.invite_community_id.map(CommunityId::new),
                    invite_community_name: r.invite_community_name,
                    call_status: r.call_status,
                    call_duration_seconds: r.call_duration_seconds,
                    created_at: r.created_at,
                    edited_at: r.edited_at,
                    reply_to: r.reply_to_message_id,
                    forwarded_from: parse_forward(r.forwarded_from.as_deref()),
                sticker: parse_sticker(r.sticker.as_deref()),
                })
                .collect()
            }
            (None, Some(a)) => {
                let cursor = self.fetch_cursor(user_a, user_b, a).await?;
                let mut rows: Vec<DmMessageView> = sqlx::query!(
                    r#"
                    SELECT m.id, m.sender_id, m.recipient_id,
                           u.name AS sender_name,
                           m.content, m.invite_community_id,
                           s.name AS "invite_community_name?",
                           m.call_status, m.call_duration_seconds,
                           m.created_at, m.edited_at, m.reply_to_message_id, m.forwarded_from::text AS "forwarded_from?", m.sticker::text AS "sticker?"
                    FROM dm_messages m
                    INNER JOIN messaging_user_profiles u ON u.user_id = m.sender_id
                    LEFT JOIN communities s ON s.id = m.invite_community_id
                    WHERE LEAST(m.sender_id, m.recipient_id) = LEAST($1::uuid, $2::uuid)
                      AND GREATEST(m.sender_id, m.recipient_id) = GREATEST($1::uuid, $2::uuid)
                      AND NOT EXISTS (SELECT 1 FROM hidden_messages h WHERE h.user_id = $1 AND h.message_id = m.id)
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
                    invite_community_id: r.invite_community_id.map(CommunityId::new),
                    invite_community_name: r.invite_community_name,
                    call_status: r.call_status,
                    call_duration_seconds: r.call_duration_seconds,
                    created_at: r.created_at,
                    edited_at: r.edited_at,
                    reply_to: r.reply_to_message_id,
                    forwarded_from: parse_forward(r.forwarded_from.as_deref()),
                sticker: parse_sticker(r.sticker.as_deref()),
                })
                .collect();
                rows.reverse();
                rows
            }
            (None, None) => sqlx::query!(
                r#"
                SELECT m.id, m.sender_id, m.recipient_id,
                       u.name AS sender_name,
                       m.content, m.invite_community_id,
                       s.name AS "invite_community_name?",
                       m.call_status, m.call_duration_seconds,
                       m.created_at, m.edited_at, m.reply_to_message_id, m.forwarded_from::text AS "forwarded_from?", m.sticker::text AS "sticker?"
                FROM dm_messages m
                INNER JOIN messaging_user_profiles u ON u.user_id = m.sender_id
                LEFT JOIN communities s ON s.id = m.invite_community_id
                WHERE LEAST(m.sender_id, m.recipient_id) = LEAST($1::uuid, $2::uuid)
                  AND GREATEST(m.sender_id, m.recipient_id) = GREATEST($1::uuid, $2::uuid)
                  AND NOT EXISTS (SELECT 1 FROM hidden_messages h WHERE h.user_id = $1 AND h.message_id = m.id)
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
                invite_community_id: r.invite_community_id.map(CommunityId::new),
                invite_community_name: r.invite_community_name,
                call_status: r.call_status,
                call_duration_seconds: r.call_duration_seconds,
                created_at: r.created_at,
                edited_at: r.edited_at,
                reply_to: r.reply_to_message_id,
                forwarded_from: parse_forward(r.forwarded_from.as_deref()),
                sticker: parse_sticker(r.sticker.as_deref()),
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
            FROM dm_messages m
            WHERE LEAST(m.sender_id, m.recipient_id) = LEAST($1::uuid, $2::uuid)
              AND GREATEST(m.sender_id, m.recipient_id) = GREATEST($1::uuid, $2::uuid)
              AND NOT EXISTS (SELECT 1 FROM hidden_messages h WHERE h.user_id = $1 AND h.message_id = m.id)
              AND m.search_tsv @@ plainto_tsquery('simple', $3)
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
                   m.content, m.invite_community_id,
                   s.name AS "invite_community_name?",
                   m.call_status, m.call_duration_seconds,
                   m.created_at, m.edited_at, m.reply_to_message_id, m.forwarded_from::text AS "forwarded_from?", m.sticker::text AS "sticker?"
            FROM dm_messages m
            INNER JOIN messaging_user_profiles u ON u.user_id = m.sender_id
            LEFT JOIN communities s ON s.id = m.invite_community_id
            WHERE LEAST(m.sender_id, m.recipient_id) = LEAST($1::uuid, $2::uuid)
              AND GREATEST(m.sender_id, m.recipient_id) = GREATEST($1::uuid, $2::uuid)
              AND NOT EXISTS (SELECT 1 FROM hidden_messages h WHERE h.user_id = $1 AND h.message_id = m.id)
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
                invite_community_id: r.invite_community_id.map(CommunityId::new),
                invite_community_name: r.invite_community_name,
                call_status: r.call_status,
                call_duration_seconds: r.call_duration_seconds,
                created_at: r.created_at,
                edited_at: r.edited_at,
                reply_to: r.reply_to_message_id,
                forwarded_from: parse_forward(r.forwarded_from.as_deref()),
                sticker: parse_sticker(r.sticker.as_deref()),
            })
            .collect();
        Ok(PaginatedResult::new(items, total, pagination))
    }

    async fn has_conversation(
        &self,
        user_id: &UserId,
        peer_id: &UserId,
    ) -> Result<bool, RepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM dm_messages
                WHERE (sender_id = $1 AND recipient_id = $2)
                   OR (sender_id = $2 AND recipient_id = $1)
            ) AS "exists!"
            "#,
            user_id.uuid(),
            peer_id.uuid(),
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.exists)
    }

    async fn has_pending_community_invite(
        &self,
        sender_id: &UserId,
        recipient_id: &UserId,
        community_id: &CommunityId,
    ) -> Result<bool, RepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM dm_messages
                WHERE sender_id = $1
                  AND recipient_id = $2
                  AND invite_community_id = $3
            ) AS "exists!"
            "#,
            sender_id.uuid(),
            recipient_id.uuid(),
            community_id.uuid(),
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.exists)
    }

    async fn find_invite_recipients(
        &self,
        sender_id: &UserId,
        community_id: &CommunityId,
    ) -> Result<Vec<UserId>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT DISTINCT recipient_id
            FROM dm_messages
            WHERE sender_id = $1 AND invite_community_id = $2
            "#,
            sender_id.uuid(),
            community_id.uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows
            .into_iter()
            .map(|r| UserId::new(r.recipient_id))
            .collect())
    }

    async fn peers_with_history(
        &self,
        user_id: &UserId,
        candidates: &[UserId],
    ) -> Result<Vec<UserId>, RepositoryError> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        let ids: Vec<Uuid> = candidates.iter().map(|c| c.uuid()).collect();
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT peer_id FROM dm_conversations WHERE user_id = $1 AND peer_id = ANY($2)
            "#,
        )
        .bind(user_id.uuid())
        .bind(&ids)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows.into_iter().map(|(id,)| UserId::new(id)).collect())
    }

    async fn list_conversation_previews(
        &self,
        user_id: &UserId,
        cursor: Option<&KeysetCursor>,
        limit: i64,
    ) -> Result<Vec<DmConversationPreview>, RepositoryError> {
        let cursor_at = cursor.map(|c| c.at);
        let cursor_id = cursor.and_then(|c| Uuid::parse_str(&c.key).ok());
        #[allow(clippy::type_complexity)]
        let rows: Vec<(Uuid, Uuid, String, Uuid, String, Option<String>, Option<Uuid>, bool, Option<i32>, Option<i32>, Option<String>, DateTime<Utc>)> =
            sqlx::query_as(
                r#"
                SELECT e.message_id, c.peer_id, u.name AS peer_name, m.sender_id, m.content, m.call_status,
                       m.invite_community_id,
                       EXISTS(
                           SELECT 1 FROM dm_message_attachments att WHERE att.dm_message_id = m.id
                       ) AS has_attachments,
                       (
                           SELECT att.voice_duration_ms FROM dm_message_attachments att
                           WHERE att.dm_message_id = m.id AND att.voice_duration_ms IS NOT NULL
                           LIMIT 1
                       ) AS voice_duration_ms,
                       (
                           SELECT att.video_duration_ms FROM dm_message_attachments att
                           WHERE att.dm_message_id = m.id AND att.video_duration_ms IS NOT NULL
                           LIMIT 1
                       ) AS video_note_duration_ms,
                       m.sticker->>'emoji' AS sticker_emoji,
                       e.created_at
                FROM dm_conversations c
                CROSS JOIN LATERAL (
                    SELECT c.last_message_id AS message_id, c.last_created_at AS created_at
                    WHERE NOT EXISTS (
                        SELECT 1 FROM hidden_messages h
                        WHERE h.user_id = $1 AND h.message_id = c.last_message_id
                    )
                    UNION ALL
                    (
                        SELECT v.id, v.created_at
                        FROM dm_messages v
                        WHERE EXISTS (
                                SELECT 1 FROM hidden_messages h
                                WHERE h.user_id = $1 AND h.message_id = c.last_message_id
                            )
                          AND LEAST(v.sender_id, v.recipient_id) = LEAST($1::uuid, c.peer_id)
                          AND GREATEST(v.sender_id, v.recipient_id) = GREATEST($1::uuid, c.peer_id)
                          AND NOT EXISTS (
                              SELECT 1 FROM hidden_messages h
                              WHERE h.user_id = $1 AND h.message_id = v.id
                          )
                        ORDER BY v.created_at DESC, v.id DESC
                        LIMIT 1
                    )
                ) e
                INNER JOIN dm_messages m ON m.id = e.message_id
                INNER JOIN messaging_user_profiles u ON u.user_id = c.peer_id
                WHERE c.user_id = $1
                  AND NOT EXISTS (
                      SELECT 1 FROM user_blocks b
                      WHERE (b.blocker_id = $1 AND b.blocked_id = c.peer_id)
                         OR (b.blocker_id = c.peer_id AND b.blocked_id = $1)
                  )
                  AND ($2::timestamptz IS NULL OR (e.created_at, e.message_id) < ($2, $3::uuid))
                ORDER BY e.created_at DESC, e.message_id DESC
                LIMIT $4
                "#,
            )
            .bind(user_id.uuid())
            .bind(cursor_at)
            .bind(cursor_id)
            .bind(limit + 1)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        Ok(rows
            .into_iter()
            .map(
                |(message_id, peer_id, peer_name, sender_id, content, call_status, invite_community_id, has_attachments, voice_duration_ms, video_note_duration_ms, sticker_emoji, created_at)| {
                    DmConversationPreview {
                        message_id,
                        peer_id,
                        peer_name,
                        sender_id,
                        content,
                        call_status,
                        invite_community_id,
                        has_attachments,
                        voice_duration_ms,
                        video_note_duration_ms,
                        sticker_emoji,
                        created_at,
                    }
                },
            )
            .collect())
    }
}

pub(crate) async fn insert_dm_links(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    dm_message_id: Uuid,
    attachments: &[AttachmentLink],
) -> Result<(), RepositoryError> {
    for link in attachments {
        sqlx::query!(
            r#"
            INSERT INTO dm_message_attachments
                (dm_message_id, attachment_id, file_name, content_type, size_bytes, storage_key, created_at,
                 voice_duration_ms, voice_waveform, video_duration_ms, video_size, video_poster)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
            dm_message_id,
            link.id,
            link.file_name,
            link.content_type,
            link.size_bytes,
            link.storage_key,
            link.created_at,
            link.voice.as_ref().map(|v| v.duration_ms),
            link.voice.as_ref().map(|v| v.waveform.as_slice()),
            link.video_note.as_ref().map(|v| v.duration_ms),
            link.video_note.as_ref().map(|v| v.size),
            link.video_note.as_ref().map(|v| v.poster.as_slice()),
        )
        .execute(&mut **tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
    }
    Ok(())
}

fn parse_sticker(raw: Option<&str>) -> Option<domain::sticker::StickerSnapshot> {
    super::message::parse_sticker(raw)
}

fn parse_forward(raw: Option<&str>) -> Option<messaging_domain::message::forward::ForwardInfo> {
    let value: serde_json::Value = serde_json::from_str(raw?).ok()?;
    messaging_domain::message::forward::ForwardInfo::from_json(&value)
}
