use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use uuid::Uuid;

use crate::search::SearchPlace;

pub const SEARCH_HISTORY_LIMIT: i64 = 20;
pub const HISTORY_QUERY_MIN_CHARS: usize = 2;
pub const HISTORY_QUERY_MAX_CHARS: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryEntry {
    Query(String),
    Person(Uuid),
    Dm(Uuid),
    Community(Uuid),
    Channel(Uuid),
    Message(Uuid),
}

impl HistoryEntry {
    pub fn parse(kind: &str, query: Option<&str>, target: Option<Uuid>) -> Option<Self> {
        match (kind, query, target) {
            ("query", Some(text), None) => {
                let trimmed = text.trim();
                let chars = trimmed.chars().count();
                (HISTORY_QUERY_MIN_CHARS..=HISTORY_QUERY_MAX_CHARS)
                    .contains(&chars)
                    .then(|| Self::Query(trimmed.to_string()))
            }
            ("person", None, Some(id)) => Some(Self::Person(id)),
            ("dm", None, Some(id)) => Some(Self::Dm(id)),
            ("community", None, Some(id)) => Some(Self::Community(id)),
            ("channel", None, Some(id)) => Some(Self::Channel(id)),
            ("message", None, Some(id)) => Some(Self::Message(id)),
            _ => None,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::Query(_) => "query",
            Self::Person(_) => "person",
            Self::Dm(_) => "dm",
            Self::Community(_) => "community",
            Self::Channel(_) => "channel",
            Self::Message(_) => "message",
        }
    }

    pub fn key(&self) -> String {
        match self {
            Self::Query(text) => text.to_lowercase(),
            Self::Person(id) | Self::Dm(id) | Self::Community(id) | Self::Channel(id) | Self::Message(id) => {
                id.to_string()
            }
        }
    }

    pub fn query(&self) -> Option<&str> {
        match self {
            Self::Query(text) => Some(text),
            _ => None,
        }
    }

    pub fn target(&self) -> Option<Uuid> {
        match self {
            Self::Query(_) => None,
            Self::Person(id) | Self::Dm(id) | Self::Community(id) | Self::Channel(id) | Self::Message(id) => Some(*id),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryTarget {
    Query {
        text: String,
    },
    Person {
        user_id: Uuid,
        name: String,
    },
    Dm {
        user_id: Uuid,
        name: String,
    },
    Community {
        community_id: Uuid,
        name: String,
    },
    Channel {
        community_id: Uuid,
        community_name: String,
        channel_id: Uuid,
        name: String,
        voice: bool,
    },
    Message {
        message_id: Uuid,
        author_id: Uuid,
        author_name: String,
        snippet: String,
        created_at: DateTime<Utc>,
        place: SearchPlace,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryItem {
    pub id: Uuid,
    pub used_at: DateTime<Utc>,
    pub target: HistoryTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordedEntry {
    pub id: Uuid,
    pub used_at: DateTime<Utc>,
}

#[async_trait]
pub trait SearchHistoryRepository: Send + Sync {
    async fn list(&self, user: &UserId) -> Result<Vec<HistoryItem>, RepositoryError>;
    async fn record(&self, user: &UserId, entry: &HistoryEntry, at: DateTime<Utc>) -> Result<RecordedEntry, RepositoryError>;
    async fn remove(&self, user: &UserId, id: Uuid) -> Result<(), RepositoryError>;
    async fn clear(&self, user: &UserId) -> Result<(), RepositoryError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queries_are_trimmed_bounded_and_keyed_case_insensitively() {
        let entry = HistoryEntry::parse("query", Some("  Rust Async "), None).unwrap();
        assert_eq!(entry, HistoryEntry::Query("Rust Async".to_string()));
        assert_eq!(entry.key(), "rust async");
        assert_eq!(HistoryEntry::parse("query", Some("a"), None), None);
        assert_eq!(HistoryEntry::parse("query", Some(&"x".repeat(101)), None), None);
    }

    #[test]
    fn targets_need_an_id_and_queries_need_text() {
        let id = Uuid::new_v4();
        assert_eq!(HistoryEntry::parse("person", None, Some(id)), Some(HistoryEntry::Person(id)));
        assert_eq!(HistoryEntry::parse("message", None, Some(id)).unwrap().key(), id.to_string());
        assert_eq!(HistoryEntry::parse("person", Some("bob"), Some(id)), None);
        assert_eq!(HistoryEntry::parse("query", None, Some(id)), None);
        assert_eq!(HistoryEntry::parse("channel", None, None), None);
        assert_eq!(HistoryEntry::parse("file", None, Some(id)), None);
    }
}
