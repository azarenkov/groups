use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::{
    attachment::{AttachmentLink, VideoNoteMeta, VoiceMeta},
    user::id::UserId,
};
use messaging_domain::shared::{
    LINK_PATTERN, SharedContent, SharedContentRepository, SharedCursor, SharedItem, SharedItemKey, SharedKind,
    SharedScope,
};
use ports::repository::RepositoryError;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

pub struct PostgresSharedContentRepository {
    pool: Pool<Postgres>,
}

impl PostgresSharedContentRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

fn like_pattern(query: Option<&str>) -> String {
    let Some(query) = query else {
        return String::new();
    };
    let mut out = String::with_capacity(query.len() + 2);
    out.push('%');
    for c in query.chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('%');
    out
}

struct AttachmentRow {
    message_id: Uuid,
    author_id: Uuid,
    message_created_at: DateTime<Utc>,
    attachment_id: Uuid,
    file_name: String,
    content_type: String,
    size_bytes: i64,
    storage_key: String,
    created_at: DateTime<Utc>,
    voice_duration_ms: Option<i32>,
    voice_waveform: Option<Vec<u8>>,
    video_duration_ms: Option<i32>,
    video_size: Option<i32>,
    video_poster: Option<Vec<u8>>,
}

impl From<AttachmentRow> for SharedItem {
    fn from(r: AttachmentRow) -> Self {
        Self {
            message_id: r.message_id,
            author_id: UserId::new(r.author_id),
            created_at: r.message_created_at,
            cursor: SharedCursor {
                created_at: r.message_created_at,
                message_id: r.message_id,
                item: SharedItemKey::Attachment(r.attachment_id),
            },
            content: SharedContent::Attachment(AttachmentLink {
                id: r.attachment_id,
                file_name: r.file_name,
                content_type: r.content_type,
                size_bytes: r.size_bytes,
                storage_key: r.storage_key,
                created_at: r.created_at,
                voice: VoiceMeta::from_columns(r.voice_duration_ms, r.voice_waveform),
                video_note: VideoNoteMeta::from_columns(r.video_duration_ms, r.video_size, r.video_poster),
            }),
        }
    }
}

struct LinkRow {
    message_id: Uuid,
    author_id: Uuid,
    created_at: DateTime<Utc>,
    url: String,
    ordinal: i64,
}

impl From<LinkRow> for SharedItem {
    fn from(r: LinkRow) -> Self {
        Self {
            message_id: r.message_id,
            author_id: UserId::new(r.author_id),
            created_at: r.created_at,
            cursor: SharedCursor {
                created_at: r.created_at,
                message_id: r.message_id,
                item: SharedItemKey::Link(r.ordinal),
            },
            content: SharedContent::Link(r.url),
        }
    }
}

fn split_cursor(before: Option<&SharedCursor>) -> (Option<DateTime<Utc>>, Option<Uuid>, Option<Uuid>, Option<i64>) {
    match before {
        None => (None, None, None, None),
        Some(cursor) => match cursor.item {
            SharedItemKey::Attachment(id) => (Some(cursor.created_at), Some(cursor.message_id), Some(id), None),
            SharedItemKey::Link(ordinal) => (Some(cursor.created_at), Some(cursor.message_id), None, Some(ordinal)),
        },
    }
}

#[async_trait]
impl SharedContentRepository for PostgresSharedContentRepository {
    async fn list_shared(
        &self,
        scope: &SharedScope,
        viewer: &UserId,
        kind: SharedKind,
        query: Option<&str>,
        before: Option<&SharedCursor>,
        limit: i64,
    ) -> Result<Vec<SharedItem>, RepositoryError> {
        let pattern = like_pattern(query);
        let (before_at, before_message, before_attachment, before_ordinal) = split_cursor(before);
        let items: Vec<SharedItem> = match (scope, kind) {
            (SharedScope::Channel(channel_id), SharedKind::Links) => sqlx::query_as!(
                LinkRow,
                r#"
                SELECT m.id AS "message_id!", m.author_id AS "author_id!", m.created_at AS "created_at!",
                       l.found[1] AS "url!", l.ordinal AS "ordinal!"
                FROM messages m
                CROSS JOIN LATERAL regexp_matches(m.content, $3, 'g') WITH ORDINALITY AS l(found, ordinal)
                WHERE m.channel_id = $1
                  AND m.kind = 'user'
                  AND m.content ~ 'https?://'
                  AND NOT EXISTS (SELECT 1 FROM hidden_messages h WHERE h.user_id = $2 AND h.message_id = m.id)
                  AND ($4 = '' OR lower(l.found[1]) LIKE $4 ESCAPE '\')
                  AND ($5::timestamptz IS NULL OR (m.created_at, m.id, l.ordinal) < ($5, $6, $7))
                ORDER BY m.created_at DESC, m.id DESC, l.ordinal DESC
                LIMIT $8
                "#,
                channel_id.uuid(),
                viewer.uuid(),
                LINK_PATTERN,
                pattern,
                before_at,
                before_message,
                before_ordinal,
                limit,
            )
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error)?
            .into_iter()
            .map(SharedItem::from)
            .collect(),
            (SharedScope::Channel(channel_id), kind) => sqlx::query_as!(
                AttachmentRow,
                r#"
                SELECT a.message_id, m.author_id, m.created_at AS message_created_at, a.attachment_id,
                       a.file_name, a.content_type, a.size_bytes, a.storage_key, a.created_at,
                       a.voice_duration_ms, a.voice_waveform, a.video_duration_ms, a.video_size, a.video_poster
                FROM message_attachments a
                JOIN messages m ON m.id = a.message_id
                WHERE m.channel_id = $1
                  AND m.kind = 'user'
                  AND NOT EXISTS (SELECT 1 FROM hidden_messages h WHERE h.user_id = $2 AND h.message_id = m.id)
                  AND CASE $3::text
                        WHEN 'voice' THEN a.voice_duration_ms IS NOT NULL OR a.video_duration_ms IS NOT NULL
                        WHEN 'media' THEN a.voice_duration_ms IS NULL AND a.video_duration_ms IS NULL
                             AND (a.content_type LIKE 'image/%' OR a.content_type LIKE 'video/%')
                        ELSE a.voice_duration_ms IS NULL AND a.video_duration_ms IS NULL
                             AND a.content_type NOT LIKE 'image/%' AND a.content_type NOT LIKE 'video/%'
                      END
                  AND ($4 = '' OR lower(a.file_name) LIKE $4 ESCAPE '\')
                  AND ($5::timestamptz IS NULL OR (m.created_at, m.id, a.attachment_id) < ($5, $6, $7))
                ORDER BY m.created_at DESC, m.id DESC, a.attachment_id DESC
                LIMIT $8
                "#,
                channel_id.uuid(),
                viewer.uuid(),
                kind.as_str(),
                pattern,
                before_at,
                before_message,
                before_attachment,
                limit,
            )
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error)?
            .into_iter()
            .map(SharedItem::from)
            .collect(),
            (SharedScope::Dm { user_a, user_b }, SharedKind::Links) => sqlx::query_as!(
                LinkRow,
                r#"
                SELECT m.id AS "message_id!", m.sender_id AS "author_id!", m.created_at AS "created_at!",
                       l.found[1] AS "url!", l.ordinal AS "ordinal!"
                FROM dm_messages m
                CROSS JOIN LATERAL regexp_matches(m.content, $4, 'g') WITH ORDINALITY AS l(found, ordinal)
                WHERE LEAST(m.sender_id, m.recipient_id) = LEAST($1::uuid, $2::uuid)
                  AND GREATEST(m.sender_id, m.recipient_id) = GREATEST($1::uuid, $2::uuid)
                  AND m.content ~ 'https?://'
                  AND NOT EXISTS (SELECT 1 FROM hidden_messages h WHERE h.user_id = $3 AND h.message_id = m.id)
                  AND ($5 = '' OR lower(l.found[1]) LIKE $5 ESCAPE '\')
                  AND ($6::timestamptz IS NULL OR (m.created_at, m.id, l.ordinal) < ($6, $7, $8))
                ORDER BY m.created_at DESC, m.id DESC, l.ordinal DESC
                LIMIT $9
                "#,
                user_a.uuid(),
                user_b.uuid(),
                viewer.uuid(),
                LINK_PATTERN,
                pattern,
                before_at,
                before_message,
                before_ordinal,
                limit,
            )
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error)?
            .into_iter()
            .map(SharedItem::from)
            .collect(),
            (SharedScope::Dm { user_a, user_b }, kind) => sqlx::query_as!(
                AttachmentRow,
                r#"
                SELECT a.dm_message_id AS message_id, m.sender_id AS author_id, m.created_at AS message_created_at,
                       a.attachment_id, a.file_name, a.content_type, a.size_bytes, a.storage_key, a.created_at,
                       a.voice_duration_ms, a.voice_waveform, a.video_duration_ms, a.video_size, a.video_poster
                FROM dm_message_attachments a
                JOIN dm_messages m ON m.id = a.dm_message_id
                WHERE LEAST(m.sender_id, m.recipient_id) = LEAST($1::uuid, $2::uuid)
                  AND GREATEST(m.sender_id, m.recipient_id) = GREATEST($1::uuid, $2::uuid)
                  AND NOT EXISTS (SELECT 1 FROM hidden_messages h WHERE h.user_id = $3 AND h.message_id = m.id)
                  AND CASE $4::text
                        WHEN 'voice' THEN a.voice_duration_ms IS NOT NULL OR a.video_duration_ms IS NOT NULL
                        WHEN 'media' THEN a.voice_duration_ms IS NULL AND a.video_duration_ms IS NULL
                             AND (a.content_type LIKE 'image/%' OR a.content_type LIKE 'video/%')
                        ELSE a.voice_duration_ms IS NULL AND a.video_duration_ms IS NULL
                             AND a.content_type NOT LIKE 'image/%' AND a.content_type NOT LIKE 'video/%'
                      END
                  AND ($5 = '' OR lower(a.file_name) LIKE $5 ESCAPE '\')
                  AND ($6::timestamptz IS NULL OR (m.created_at, m.id, a.attachment_id) < ($6, $7, $8))
                ORDER BY m.created_at DESC, m.id DESC, a.attachment_id DESC
                LIMIT $9
                "#,
                user_a.uuid(),
                user_b.uuid(),
                viewer.uuid(),
                kind.as_str(),
                pattern,
                before_at,
                before_message,
                before_attachment,
                limit,
            )
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error)?
            .into_iter()
            .map(SharedItem::from)
            .collect(),
        };
        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::like_pattern;

    #[test]
    fn like_patterns_escape_wildcards() {
        assert_eq!(like_pattern(None), "");
        assert_eq!(like_pattern(Some("50%_off\\")), "%50\\%\\_off\\\\%");
    }
}
