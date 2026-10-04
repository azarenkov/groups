use std::cmp::Ordering;

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use domain::{attachment::AttachmentLink, user::id::UserId};
use ports::repository::RepositoryError;
use uuid::Uuid;

pub const SEARCH_MIN_CHARS: usize = 2;
pub const SEARCH_MAX_CHARS: usize = 100;
pub const SEARCH_PAGE_DEFAULT: i64 = 20;
pub const SEARCH_PAGE_MAX: i64 = 50;
pub const SNIPPET_MAX_CHARS: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchKind {
    Messages,
    Files,
    Links,
}

impl SearchKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "messages" => Some(Self::Messages),
            "files" => Some(Self::Files),
            "links" => Some(Self::Links),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchItemKey {
    Message,
    Attachment(Uuid),
    Link(i64),
}

impl SearchItemKey {
    fn order(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Attachment(a), Self::Attachment(b)) => a.cmp(b),
            (Self::Link(a), Self::Link(b)) => a.cmp(b),
            _ => Ordering::Equal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchCursor {
    pub created_at: DateTime<Utc>,
    pub message_id: Uuid,
    pub item: SearchItemKey,
}

impl SearchCursor {
    pub fn encode(&self) -> String {
        let item = match self.item {
            SearchItemKey::Message => "0".to_string(),
            SearchItemKey::Attachment(id) => id.simple().to_string(),
            SearchItemKey::Link(ordinal) => ordinal.to_string(),
        };
        format!("{}_{}_{}", self.created_at.timestamp_micros(), self.message_id.simple(), item)
    }

    pub fn decode(raw: &str, kind: SearchKind) -> Option<Self> {
        let mut parts = raw.split('_');
        let micros: i64 = parts.next()?.parse().ok()?;
        let message_id = Uuid::parse_str(parts.next()?).ok()?;
        let item_raw = parts.next()?;
        if parts.next().is_some() {
            return None;
        }
        let item = match kind {
            SearchKind::Messages => (item_raw == "0").then_some(SearchItemKey::Message)?,
            SearchKind::Files => SearchItemKey::Attachment(Uuid::parse_str(item_raw).ok()?),
            SearchKind::Links => SearchItemKey::Link(item_raw.parse().ok().filter(|n: &i64| *n > 0)?),
        };
        Some(Self {
            created_at: Utc.timestamp_micros(micros).single()?,
            message_id,
            item,
        })
    }

    pub fn newest_first(&self, other: &Self) -> Ordering {
        other
            .created_at
            .cmp(&self.created_at)
            .then_with(|| other.message_id.cmp(&self.message_id))
            .then_with(|| other.item.order(&self.item))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchPlace {
    Channel {
        community_id: Uuid,
        community_name: String,
        channel_id: Uuid,
        channel_name: String,
    },
    Dm {
        peer_id: Uuid,
        peer_name: String,
    },
}

#[derive(Debug, Clone)]
pub enum SearchMatch {
    Message,
    Attachment(AttachmentLink),
    Link(String),
}

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub message_id: Uuid,
    pub author_id: UserId,
    pub author_name: String,
    pub forwarded_author_id: Option<UserId>,
    pub created_at: DateTime<Utc>,
    pub place: SearchPlace,
    pub snippet: String,
    pub found: SearchMatch,
    pub cursor: SearchCursor,
}

pub fn normalize_search(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let chars = trimmed.chars().count();
    (SEARCH_MIN_CHARS..=SEARCH_MAX_CHARS).contains(&chars).then(|| trimmed.to_string())
}

pub fn clamp_search_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(SEARCH_PAGE_DEFAULT).clamp(1, SEARCH_PAGE_MAX)
}

pub const SNIPPET_LEAD_CHARS: usize = 60;

fn fold(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

fn first_match(haystack: &[char], query: &str) -> Option<usize> {
    query
        .split_whitespace()
        .filter_map(|term| {
            let needle: Vec<char> = term.chars().map(fold).collect();
            if needle.is_empty() || needle.len() > haystack.len() {
                return None;
            }
            (0..=haystack.len() - needle.len())
                .find(|&start| haystack[start..start + needle.len()].iter().zip(&needle).all(|(a, b)| fold(*a) == *b))
        })
        .min()
}

pub fn snippet(content: &str, query: &str) -> String {
    let flat: Vec<char> = content.split_whitespace().collect::<Vec<_>>().join(" ").chars().collect();
    if flat.len() <= SNIPPET_MAX_CHARS {
        return flat.into_iter().collect();
    }
    let hit = first_match(&flat, query).unwrap_or(0);
    let start = hit
        .saturating_sub(SNIPPET_LEAD_CHARS)
        .min(flat.len() - SNIPPET_MAX_CHARS);
    let end = start + SNIPPET_MAX_CHARS;
    let mut out = String::new();
    if start > 0 {
        out.push('…');
    }
    out.extend(&flat[start..end]);
    if end < flat.len() {
        out.push('…');
    }
    out
}

pub fn merge_newest(mut hits: Vec<SearchHit>, limit: usize) -> Vec<SearchHit> {
    hits.sort_by(|a, b| a.cursor.newest_first(&b.cursor));
    hits.truncate(limit);
    hits
}

#[async_trait]
pub trait SearchRepository {
    async fn search(
        &self,
        viewer: &UserId,
        kind: SearchKind,
        query: &str,
        before: Option<&SearchCursor>,
        limit: i64,
    ) -> Result<Vec<SearchHit>, RepositoryError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cursor(micros: i64, item: SearchItemKey) -> SearchCursor {
        SearchCursor {
            created_at: Utc.timestamp_micros(micros).single().unwrap(),
            message_id: Uuid::from_u128(micros as u128),
            item,
        }
    }

    #[test]
    fn cursors_round_trip_and_reject_mismatched_kinds() {
        let message = cursor(1_790_000_000_000_001, SearchItemKey::Message);
        assert_eq!(SearchCursor::decode(&message.encode(), SearchKind::Messages), Some(message));
        let file = cursor(5, SearchItemKey::Attachment(Uuid::new_v4()));
        assert_eq!(SearchCursor::decode(&file.encode(), SearchKind::Files), Some(file));
        let link = cursor(7, SearchItemKey::Link(2));
        assert_eq!(SearchCursor::decode(&link.encode(), SearchKind::Links), Some(link));
        assert_eq!(SearchCursor::decode(&link.encode(), SearchKind::Messages), None);
        assert_eq!(SearchCursor::decode("garbage", SearchKind::Files), None);
    }

    #[test]
    fn queries_need_two_to_a_hundred_characters() {
        assert_eq!(normalize_search("  hi  "), Some("hi".to_string()));
        assert_eq!(normalize_search("x"), None);
        assert_eq!(normalize_search(&"x".repeat(101)), None);
    }

    #[test]
    fn snippets_are_flattened_and_shortened() {
        assert_eq!(snippet("a\n  b", "a"), "a b");
        assert_eq!(snippet("one   two\tthree", "two"), "one two three");
        assert!(snippet(&"x".repeat(500), "zz").ends_with('…'));
        assert_eq!(snippet(&"x".repeat(500), "zz").chars().count(), SNIPPET_MAX_CHARS + 1);
    }

    #[test]
    fn long_snippets_are_centered_on_the_first_matched_term() {
        let content = format!("{} Привет мир {}", "a".repeat(300), "b".repeat(300));
        let cut = snippet(&content, "zzz ПРИВ");
        assert!(cut.starts_with('…') && cut.ends_with('…'));
        assert!(cut.contains("Привет мир"));
        assert_eq!(cut.chars().count(), SNIPPET_MAX_CHARS + 2);
        let tail = format!("{} конец", "a".repeat(400));
        let end = snippet(&tail, "конец");
        assert!(end.starts_with('…') && end.ends_with("конец"));
        assert_eq!(end.chars().count(), SNIPPET_MAX_CHARS + 1);
    }

    #[test]
    fn hits_from_channels_and_dms_merge_newest_first() {
        let hit = |micros: i64| SearchHit {
            message_id: Uuid::from_u128(micros as u128),
            author_id: UserId::new(Uuid::nil()),
            author_name: String::new(),
            forwarded_author_id: None,
            created_at: Utc.timestamp_micros(micros).single().unwrap(),
            place: SearchPlace::Dm { peer_id: Uuid::nil(), peer_name: String::new() },
            snippet: String::new(),
            found: SearchMatch::Message,
            cursor: cursor(micros, SearchItemKey::Message),
        };
        let merged = merge_newest(vec![hit(1), hit(3), hit(2)], 2);
        assert_eq!(merged.iter().map(|h| h.created_at.timestamp_micros()).collect::<Vec<_>>(), vec![3, 2]);
    }
}
