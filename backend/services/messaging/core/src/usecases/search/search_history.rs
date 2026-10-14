use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use messaging_domain::search_history::{HistoryEntry, HistoryItem, RecordedEntry, SearchHistoryRepository};
use ports::repository::RepositoryError;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum SearchHistoryError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("A history entry needs a known kind with either 2 to 100 characters of query text or a target id")]
    InvalidEntry,
}

pub struct SearchHistory<R: SearchHistoryRepository> {
    repository: Arc<R>,
}

impl<R: SearchHistoryRepository> SearchHistory<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    pub async fn list(&self, user: UserId) -> Result<Vec<HistoryItem>, SearchHistoryError> {
        Ok(self.repository.list(&user).await?)
    }

    pub async fn record(
        &self,
        user: UserId,
        kind: &str,
        query: Option<&str>,
        target: Option<Uuid>,
    ) -> Result<RecordedEntry, SearchHistoryError> {
        let entry = HistoryEntry::parse(kind, query, target).ok_or(SearchHistoryError::InvalidEntry)?;
        Ok(self.repository.record(&user, &entry, Utc::now()).await?)
    }

    pub async fn remove(&self, user: UserId, id: Uuid) -> Result<(), SearchHistoryError> {
        Ok(self.repository.remove(&user, id).await?)
    }

    pub async fn clear(&self, user: UserId) -> Result<(), SearchHistoryError> {
        Ok(self.repository.clear(&user).await?)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use chrono::{DateTime, Utc};

    use super::*;

    #[derive(Default)]
    struct Recorded(Mutex<Vec<HistoryEntry>>);

    #[async_trait]
    impl SearchHistoryRepository for Recorded {
        async fn list(&self, _: &UserId) -> Result<Vec<HistoryItem>, RepositoryError> {
            Ok(Vec::new())
        }
        async fn record(&self, _: &UserId, entry: &HistoryEntry, at: DateTime<Utc>) -> Result<RecordedEntry, RepositoryError> {
            self.0.lock().unwrap().push(entry.clone());
            Ok(RecordedEntry { id: Uuid::nil(), used_at: at })
        }
        async fn remove(&self, _: &UserId, _: Uuid) -> Result<(), RepositoryError> {
            Ok(())
        }
        async fn clear(&self, _: &UserId) -> Result<(), RepositoryError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn records_valid_entries_and_rejects_malformed_ones() {
        let repository = Arc::new(Recorded::default());
        let usecase = SearchHistory::new(repository.clone());
        let user = UserId::new(Uuid::new_v4());
        let person = Uuid::new_v4();
        usecase.record(user, "query", Some(" hello "), None).await.unwrap();
        usecase.record(user, "person", None, Some(person)).await.unwrap();
        assert!(matches!(
            usecase.record(user, "person", None, None).await,
            Err(SearchHistoryError::InvalidEntry)
        ));
        assert!(matches!(
            usecase.record(user, "query", Some("x"), None).await,
            Err(SearchHistoryError::InvalidEntry)
        ));
        assert_eq!(
            *repository.0.lock().unwrap(),
            vec![HistoryEntry::Query("hello".to_string()), HistoryEntry::Person(person)]
        );
    }
}
