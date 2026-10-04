use async_trait::async_trait;
use domain::{
    attachment::{AttachmentLink, VideoNoteMeta, VoiceMeta},
    user::id::UserId,
};
use messaging_domain::{
    search::{
        SearchCursor, SearchHit, SearchItemKey, SearchKind, SearchMatch, SearchPlace, SearchRepository, merge_newest,
        snippet,
    },
    shared::LINK_PATTERN,
};
use ports::repository::RepositoryError;
use sqlx::{Pool, Postgres};

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

pub struct PostgresSearchRepository {
    pool: Pool<Postgres>,
}

impl PostgresSearchRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

fn like_pattern(query: &str) -> String {
    let mut out = String::with_capacity(query.len() + 2);
    out.push('%');
    for c in query.to_lowercase().chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('%');
    out
}

#[async_trait]
impl SearchRepository for PostgresSearchRepository {
    async fn search(
        &self,
        viewer: &UserId,
        kind: SearchKind,
        query: &str,
        before: Option<&SearchCursor>,
        limit: i64,
    ) -> Result<Vec<SearchHit>, RepositoryError> {
        let viewer_id = viewer.uuid();
        let before_at = before.map(|c| c.created_at);
        let before_message = before.map(|c| c.message_id);
        let mut hits = Vec::new();
        match kind {
            SearchKind::Messages => {
                let channel_rows = sqlx::query!(
                    r#"
                    SELECT m.id AS "message_id!", m.author_id AS "author_id!", COALESCE(u.name, '') AS "author_name!",
                           (m.forwarded_from->>'authorId')::uuid AS "forwarded_author_id?",
                           m.created_at AS "created_at!", m.content AS "content!",
                           c.id AS "channel_id!", c.name AS "channel_name!", s.id AS "community_id!", s.name AS "community_name!"
                    FROM messages m
                    JOIN channels c ON c.id = m.channel_id AND c.kind = 'text'
                    JOIN communities s ON s.id = c.community_id
                    LEFT JOIN messaging_user_profiles u ON u.user_id = m.author_id
                    WHERE m.kind = 'user'
                      AND m.search_tsv @@ to_tsquery('simple', (SELECT string_agg(quote_literal(t.lexeme) || ':*', ' & ') FROM unnest(to_tsvector('simple', $2)) AS t))
                      AND EXISTS (SELECT 1 FROM community_members cm WHERE cm.community_id = c.community_id AND cm.user_id = $1)
                      AND (c.visibility = 'public'
                           OR EXISTS (SELECT 1 FROM channel_text_participants p WHERE p.channel_id = c.id AND p.user_id = $1))
                      AND NOT EXISTS (SELECT 1 FROM hidden_messages h WHERE h.user_id = $1 AND h.message_id = m.id)
                      AND ($3::timestamptz IS NULL OR (m.created_at, m.id) < ($3, $4))
                    ORDER BY m.created_at DESC, m.id DESC
                    LIMIT $5
                    "#,
                    viewer_id,
                    query,
                    before_at,
                    before_message,
                    limit,
                )
                .fetch_all(&self.pool)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
                hits.extend(channel_rows.into_iter().map(|r| SearchHit {
                    message_id: r.message_id,
                    author_id: UserId::new(r.author_id),
                    author_name: r.author_name,
                    forwarded_author_id: r.forwarded_author_id.map(UserId::new),
                    created_at: r.created_at,
                    place: SearchPlace::Channel {
                        community_id: r.community_id,
                        community_name: r.community_name,
                        channel_id: r.channel_id,
                        channel_name: r.channel_name,
                    },
                    snippet: snippet(&r.content),
                    found: SearchMatch::Message,
                    cursor: SearchCursor { created_at: r.created_at, message_id: r.message_id, item: SearchItemKey::Message },
                }));
                let dm_rows = sqlx::query!(
                    r#"
                    SELECT m.id AS "message_id!", m.sender_id AS "author_id!", COALESCE(u.name, '') AS "author_name!",
                           (m.forwarded_from->>'authorId')::uuid AS "forwarded_author_id?",
                           m.created_at AS "created_at!", m.content AS "content!",
                           peer.id AS "peer_id!", COALESCE(pu.name, '') AS "peer_name!"
                    FROM dm_messages m
                    CROSS JOIN LATERAL (SELECT CASE WHEN m.sender_id = $1 THEN m.recipient_id ELSE m.sender_id END AS id) peer
                    LEFT JOIN messaging_user_profiles u ON u.user_id = m.sender_id
                    LEFT JOIN messaging_user_profiles pu ON pu.user_id = peer.id
                    WHERE (m.sender_id = $1 OR m.recipient_id = $1)
                      AND NOT EXISTS (
                          SELECT 1 FROM user_blocks b
                          WHERE (b.blocker_id = $1 AND b.blocked_id = peer.id)
                             OR (b.blocker_id = peer.id AND b.blocked_id = $1)
                      )
                      AND m.invite_community_id IS NULL
                      AND m.call_status IS NULL
                      AND m.search_tsv @@ to_tsquery('simple', (SELECT string_agg(quote_literal(t.lexeme) || ':*', ' & ') FROM unnest(to_tsvector('simple', $2)) AS t))
                      AND NOT EXISTS (SELECT 1 FROM hidden_messages h WHERE h.user_id = $1 AND h.message_id = m.id)
                      AND ($3::timestamptz IS NULL OR (m.created_at, m.id) < ($3, $4))
                    ORDER BY m.created_at DESC, m.id DESC
                    LIMIT $5
                    "#,
                    viewer_id,
                    query,
                    before_at,
                    before_message,
                    limit,
                )
                .fetch_all(&self.pool)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
                hits.extend(dm_rows.into_iter().map(|r| SearchHit {
                    message_id: r.message_id,
                    author_id: UserId::new(r.author_id),
                    author_name: r.author_name,
                    forwarded_author_id: r.forwarded_author_id.map(UserId::new),
                    created_at: r.created_at,
                    place: SearchPlace::Dm { peer_id: r.peer_id, peer_name: r.peer_name },
                    snippet: snippet(&r.content),
                    found: SearchMatch::Message,
                    cursor: SearchCursor { created_at: r.created_at, message_id: r.message_id, item: SearchItemKey::Message },
                }));
            }
            SearchKind::Files => {
                let pattern = like_pattern(query);
                let before_attachment = before.and_then(|c| match c.item {
                    SearchItemKey::Attachment(id) => Some(id),
                    _ => None,
                });
                let channel_rows = sqlx::query!(
                    r#"
                    SELECT m.id AS "message_id!", m.author_id AS "author_id!", COALESCE(u.name, '') AS "author_name!",
                           (m.forwarded_from->>'authorId')::uuid AS "forwarded_author_id?",
                           m.created_at AS "created_at!", m.content AS "content!",
                           c.id AS "channel_id!", c.name AS "channel_name!", s.id AS "community_id!", s.name AS "community_name!",
                           a.attachment_id AS "attachment_id!", a.file_name AS "file_name!", a.content_type AS "content_type!",
                           a.size_bytes AS "size_bytes!", a.storage_key AS "storage_key!", a.created_at AS "attached_at!"
                    FROM message_attachments a
                    JOIN messages m ON m.id = a.message_id
                    JOIN channels c ON c.id = m.channel_id AND c.kind = 'text'
                    JOIN communities s ON s.id = c.community_id
                    LEFT JOIN messaging_user_profiles u ON u.user_id = m.author_id
                    WHERE m.kind = 'user'
                      AND a.voice_duration_ms IS NULL AND a.video_duration_ms IS NULL
                      AND lower(a.file_name) LIKE $2 ESCAPE '\'
                      AND EXISTS (SELECT 1 FROM community_members cm WHERE cm.community_id = c.community_id AND cm.user_id = $1)
                      AND (c.visibility = 'public'
                           OR EXISTS (SELECT 1 FROM channel_text_participants p WHERE p.channel_id = c.id AND p.user_id = $1))
                      AND NOT EXISTS (SELECT 1 FROM hidden_messages h WHERE h.user_id = $1 AND h.message_id = m.id)
                      AND ($3::timestamptz IS NULL OR (m.created_at, m.id, a.attachment_id) < ($3, $4, $5))
                    ORDER BY m.created_at DESC, m.id DESC, a.attachment_id DESC
                    LIMIT $6
                    "#,
                    viewer_id,
                    pattern,
                    before_at,
                    before_message,
                    before_attachment,
                    limit,
                )
                .fetch_all(&self.pool)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
                hits.extend(channel_rows.into_iter().map(|r| SearchHit {
                    message_id: r.message_id,
                    author_id: UserId::new(r.author_id),
                    author_name: r.author_name,
                    forwarded_author_id: r.forwarded_author_id.map(UserId::new),
                    created_at: r.created_at,
                    place: SearchPlace::Channel {
                        community_id: r.community_id,
                        community_name: r.community_name,
                        channel_id: r.channel_id,
                        channel_name: r.channel_name,
                    },
                    snippet: snippet(&r.content),
                    found: SearchMatch::Attachment(AttachmentLink {
                        id: r.attachment_id,
                        file_name: r.file_name,
                        content_type: r.content_type,
                        size_bytes: r.size_bytes,
                        storage_key: r.storage_key,
                        created_at: r.attached_at,
                        voice: VoiceMeta::from_columns(None, None),
                        video_note: VideoNoteMeta::from_columns(None, None, None),
                    }),
                    cursor: SearchCursor {
                        created_at: r.created_at,
                        message_id: r.message_id,
                        item: SearchItemKey::Attachment(r.attachment_id),
                    },
                }));
                let dm_rows = sqlx::query!(
                    r#"
                    SELECT m.id AS "message_id!", m.sender_id AS "author_id!", COALESCE(u.name, '') AS "author_name!",
                           (m.forwarded_from->>'authorId')::uuid AS "forwarded_author_id?",
                           m.created_at AS "created_at!", m.content AS "content!",
                           peer.id AS "peer_id!", COALESCE(pu.name, '') AS "peer_name!",
                           a.attachment_id AS "attachment_id!", a.file_name AS "file_name!", a.content_type AS "content_type!",
                           a.size_bytes AS "size_bytes!", a.storage_key AS "storage_key!", a.created_at AS "attached_at!"
                    FROM dm_message_attachments a
                    JOIN dm_messages m ON m.id = a.dm_message_id
                    CROSS JOIN LATERAL (SELECT CASE WHEN m.sender_id = $1 THEN m.recipient_id ELSE m.sender_id END AS id) peer
                    LEFT JOIN messaging_user_profiles u ON u.user_id = m.sender_id
                    LEFT JOIN messaging_user_profiles pu ON pu.user_id = peer.id
                    WHERE (m.sender_id = $1 OR m.recipient_id = $1)
                      AND NOT EXISTS (
                          SELECT 1 FROM user_blocks b
                          WHERE (b.blocker_id = $1 AND b.blocked_id = peer.id)
                             OR (b.blocker_id = peer.id AND b.blocked_id = $1)
                      )
                      AND m.invite_community_id IS NULL
                      AND m.call_status IS NULL
                      AND a.voice_duration_ms IS NULL AND a.video_duration_ms IS NULL
                      AND lower(a.file_name) LIKE $2 ESCAPE '\'
                      AND NOT EXISTS (SELECT 1 FROM hidden_messages h WHERE h.user_id = $1 AND h.message_id = m.id)
                      AND ($3::timestamptz IS NULL OR (m.created_at, m.id, a.attachment_id) < ($3, $4, $5))
                    ORDER BY m.created_at DESC, m.id DESC, a.attachment_id DESC
                    LIMIT $6
                    "#,
                    viewer_id,
                    pattern,
                    before_at,
                    before_message,
                    before_attachment,
                    limit,
                )
                .fetch_all(&self.pool)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
                hits.extend(dm_rows.into_iter().map(|r| SearchHit {
                    message_id: r.message_id,
                    author_id: UserId::new(r.author_id),
                    author_name: r.author_name,
                    forwarded_author_id: r.forwarded_author_id.map(UserId::new),
                    created_at: r.created_at,
                    place: SearchPlace::Dm { peer_id: r.peer_id, peer_name: r.peer_name },
                    snippet: snippet(&r.content),
                    found: SearchMatch::Attachment(AttachmentLink {
                        id: r.attachment_id,
                        file_name: r.file_name,
                        content_type: r.content_type,
                        size_bytes: r.size_bytes,
                        storage_key: r.storage_key,
                        created_at: r.attached_at,
                        voice: VoiceMeta::from_columns(None, None),
                        video_note: VideoNoteMeta::from_columns(None, None, None),
                    }),
                    cursor: SearchCursor {
                        created_at: r.created_at,
                        message_id: r.message_id,
                        item: SearchItemKey::Attachment(r.attachment_id),
                    },
                }));
            }
            SearchKind::Links => {
                let pattern = like_pattern(query);
                let before_ordinal = before.and_then(|c| match c.item {
                    SearchItemKey::Link(ordinal) => Some(ordinal),
                    _ => None,
                });
                let channel_rows = sqlx::query!(
                    r#"
                    SELECT m.id AS "message_id!", m.author_id AS "author_id!", COALESCE(u.name, '') AS "author_name!",
                           (m.forwarded_from->>'authorId')::uuid AS "forwarded_author_id?",
                           m.created_at AS "created_at!", m.content AS "content!",
                           c.id AS "channel_id!", c.name AS "channel_name!", s.id AS "community_id!", s.name AS "community_name!",
                           l.found[1] AS "url!", l.ordinal AS "ordinal!"
                    FROM messages m
                    JOIN channels c ON c.id = m.channel_id AND c.kind = 'text'
                    JOIN communities s ON s.id = c.community_id
                    LEFT JOIN messaging_user_profiles u ON u.user_id = m.author_id
                    CROSS JOIN LATERAL regexp_matches(m.content, $2, 'g') WITH ORDINALITY AS l(found, ordinal)
                    WHERE m.kind = 'user'
                      AND m.content ~ 'https?://'
                      AND lower(l.found[1]) LIKE $3 ESCAPE '\'
                      AND EXISTS (SELECT 1 FROM community_members cm WHERE cm.community_id = c.community_id AND cm.user_id = $1)
                      AND (c.visibility = 'public'
                           OR EXISTS (SELECT 1 FROM channel_text_participants p WHERE p.channel_id = c.id AND p.user_id = $1))
                      AND NOT EXISTS (SELECT 1 FROM hidden_messages h WHERE h.user_id = $1 AND h.message_id = m.id)
                      AND ($4::timestamptz IS NULL OR (m.created_at, m.id, l.ordinal) < ($4, $5, $6))
                    ORDER BY m.created_at DESC, m.id DESC, l.ordinal DESC
                    LIMIT $7
                    "#,
                    viewer_id,
                    LINK_PATTERN,
                    pattern,
                    before_at,
                    before_message,
                    before_ordinal,
                    limit,
                )
                .fetch_all(&self.pool)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
                hits.extend(channel_rows.into_iter().map(|r| SearchHit {
                    message_id: r.message_id,
                    author_id: UserId::new(r.author_id),
                    author_name: r.author_name,
                    forwarded_author_id: r.forwarded_author_id.map(UserId::new),
                    created_at: r.created_at,
                    place: SearchPlace::Channel {
                        community_id: r.community_id,
                        community_name: r.community_name,
                        channel_id: r.channel_id,
                        channel_name: r.channel_name,
                    },
                    snippet: snippet(&r.content),
                    found: SearchMatch::Link(r.url),
                    cursor: SearchCursor {
                        created_at: r.created_at,
                        message_id: r.message_id,
                        item: SearchItemKey::Link(r.ordinal),
                    },
                }));
                let dm_rows = sqlx::query!(
                    r#"
                    SELECT m.id AS "message_id!", m.sender_id AS "author_id!", COALESCE(u.name, '') AS "author_name!",
                           (m.forwarded_from->>'authorId')::uuid AS "forwarded_author_id?",
                           m.created_at AS "created_at!", m.content AS "content!",
                           peer.id AS "peer_id!", COALESCE(pu.name, '') AS "peer_name!",
                           l.found[1] AS "url!", l.ordinal AS "ordinal!"
                    FROM dm_messages m
                    CROSS JOIN LATERAL (SELECT CASE WHEN m.sender_id = $1 THEN m.recipient_id ELSE m.sender_id END AS id) peer
                    LEFT JOIN messaging_user_profiles u ON u.user_id = m.sender_id
                    LEFT JOIN messaging_user_profiles pu ON pu.user_id = peer.id
                    CROSS JOIN LATERAL regexp_matches(m.content, $2, 'g') WITH ORDINALITY AS l(found, ordinal)
                    WHERE (m.sender_id = $1 OR m.recipient_id = $1)
                      AND NOT EXISTS (
                          SELECT 1 FROM user_blocks b
                          WHERE (b.blocker_id = $1 AND b.blocked_id = peer.id)
                             OR (b.blocker_id = peer.id AND b.blocked_id = $1)
                      )
                      AND m.invite_community_id IS NULL
                      AND m.call_status IS NULL
                      AND m.content ~ 'https?://'
                      AND lower(l.found[1]) LIKE $3 ESCAPE '\'
                      AND NOT EXISTS (SELECT 1 FROM hidden_messages h WHERE h.user_id = $1 AND h.message_id = m.id)
                      AND ($4::timestamptz IS NULL OR (m.created_at, m.id, l.ordinal) < ($4, $5, $6))
                    ORDER BY m.created_at DESC, m.id DESC, l.ordinal DESC
                    LIMIT $7
                    "#,
                    viewer_id,
                    LINK_PATTERN,
                    pattern,
                    before_at,
                    before_message,
                    before_ordinal,
                    limit,
                )
                .fetch_all(&self.pool)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
                hits.extend(dm_rows.into_iter().map(|r| SearchHit {
                    message_id: r.message_id,
                    author_id: UserId::new(r.author_id),
                    author_name: r.author_name,
                    forwarded_author_id: r.forwarded_author_id.map(UserId::new),
                    created_at: r.created_at,
                    place: SearchPlace::Dm { peer_id: r.peer_id, peer_name: r.peer_name },
                    snippet: snippet(&r.content),
                    found: SearchMatch::Link(r.url),
                    cursor: SearchCursor {
                        created_at: r.created_at,
                        message_id: r.message_id,
                        item: SearchItemKey::Link(r.ordinal),
                    },
                }));
            }
        }
        Ok(merge_newest(hits, limit as usize))
    }
}

#[cfg(test)]
mod tests {
    use super::like_pattern;

    #[test]
    fn like_patterns_escape_wildcards_and_lowercase() {
        assert_eq!(like_pattern("Re_port%"), "%re\\_port\\%%");
    }
}
