use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ports::repository::RepositoryError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feed {
    Inbox,
    CallLog,
}

impl Feed {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Inbox => "inbox",
            Self::CallLog => "call_log",
        }
    }
}

#[async_trait]
pub trait FeedStarts: Send + Sync {
    async fn started_at(&self, feed: Feed) -> Result<DateTime<Utc>, RepositoryError>;
}
