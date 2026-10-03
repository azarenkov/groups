use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use domain::{attachment::AttachmentLink, user::id::UserId};
use ports::repository::RepositoryError;
use uuid::Uuid;

use crate::channel::id::ChannelId;

pub const SHARED_PAGE_DEFAULT: i64 = 30;
pub const SHARED_PAGE_MAX: i64 = 60;
pub const SHARED_QUERY_MAX_CHARS: usize = 100;
pub const LINK_PATTERN: &str = r#"(https?://[^[:space:]<>"')\]}]+[^[:space:]<>"')\]}.,;:!?])"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedKind {
    Media,
    Files,
    Voice,
    Links,
}

impl SharedKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "media" => Some(Self::Media),
            "files" => Some(Self::Files),
            "voice" => Some(Self::Voice),
            "links" => Some(Self::Links),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Media => "media",
            Self::Files => "files",
            Self::Voice => "voice",
            Self::Links => "links",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedCursor {
    pub created_at: DateTime<Utc>,
    pub message_id: Uuid,
    pub item: SharedItemKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedItemKey {
    Attachment(Uuid),
    Link(i64),
}

impl SharedCursor {
    pub fn encode(&self) -> String {
        let item = match self.item {
            SharedItemKey::Attachment(id) => id.simple().to_string(),
            SharedItemKey::Link(ordinal) => ordinal.to_string(),
        };
        format!("{}_{}_{}", self.created_at.timestamp_micros(), self.message_id.simple(), item)
    }

    pub fn decode(raw: &str, kind: SharedKind) -> Option<Self> {
        let mut parts = raw.split('_');
        let micros: i64 = parts.next()?.parse().ok()?;
        let message_id = Uuid::parse_str(parts.next()?).ok()?;
        let item_raw = parts.next()?;
        if parts.next().is_some() {
            return None;
        }
        let item = if kind == SharedKind::Links {
            SharedItemKey::Link(item_raw.parse().ok().filter(|n: &i64| *n > 0)?)
        } else {
            SharedItemKey::Attachment(Uuid::parse_str(item_raw).ok()?)
        };
        let created_at = Utc.timestamp_micros(micros).single()?;
        Some(Self { created_at, message_id, item })
    }
}

#[derive(Debug, Clone)]
pub enum SharedContent {
    Attachment(AttachmentLink),
    Link(String),
}

#[derive(Debug, Clone)]
pub struct SharedItem {
    pub message_id: Uuid,
    pub author_id: UserId,
    pub created_at: DateTime<Utc>,
    pub content: SharedContent,
    pub cursor: SharedCursor,
}

pub fn normalize_query(raw: Option<&str>) -> Option<String> {
    let trimmed = raw?.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.chars().take(SHARED_QUERY_MAX_CHARS).collect::<String>().to_lowercase())
}

pub fn clamp_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(SHARED_PAGE_DEFAULT).clamp(1, SHARED_PAGE_MAX)
}

#[derive(Debug, Clone)]
pub enum SharedScope {
    Channel(ChannelId),
    Dm { user_a: UserId, user_b: UserId },
}

#[async_trait]
pub trait SharedContentRepository {
    async fn list_shared(
        &self,
        scope: &SharedScope,
        viewer: &UserId,
        kind: SharedKind,
        query: Option<&str>,
        before: Option<&SharedCursor>,
        limit: i64,
    ) -> Result<Vec<SharedItem>, RepositoryError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursors_round_trip_for_each_kind() {
        let created_at = Utc.timestamp_micros(1_790_000_000_123_456).single().unwrap();
        let message_id = Uuid::new_v4();
        let attachment = SharedCursor { created_at, message_id, item: SharedItemKey::Attachment(Uuid::new_v4()) };
        assert_eq!(SharedCursor::decode(&attachment.encode(), SharedKind::Files), Some(attachment));
        let link = SharedCursor { created_at, message_id, item: SharedItemKey::Link(3) };
        assert_eq!(SharedCursor::decode(&link.encode(), SharedKind::Links), Some(link));
    }

    #[test]
    fn malformed_cursors_are_rejected() {
        for raw in ["", "abc", "1_2_3", "1_00000000000000000000000000000000_0", "1_00000000000000000000000000000000_x_y"] {
            assert_eq!(SharedCursor::decode(raw, SharedKind::Links), None, "{raw}");
        }
        let link = SharedCursor {
            created_at: Utc::now(),
            message_id: Uuid::new_v4(),
            item: SharedItemKey::Link(1),
        };
        assert_eq!(SharedCursor::decode(&link.encode(), SharedKind::Media), None);
    }

    #[test]
    fn queries_are_trimmed_lowercased_and_bounded() {
        assert_eq!(normalize_query(Some("  Report.PDF ")), Some("report.pdf".to_string()));
        assert_eq!(normalize_query(Some("   ")), None);
        assert_eq!(normalize_query(None), None);
        assert_eq!(normalize_query(Some(&"x".repeat(500))).unwrap().len(), SHARED_QUERY_MAX_CHARS);
    }

    #[test]
    fn limits_are_clamped() {
        assert_eq!(clamp_limit(None), SHARED_PAGE_DEFAULT);
        assert_eq!(clamp_limit(Some(0)), 1);
        assert_eq!(clamp_limit(Some(1000)), SHARED_PAGE_MAX);
    }
}
