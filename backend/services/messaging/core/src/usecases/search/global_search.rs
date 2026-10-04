use std::sync::Arc;

use domain::user::id::UserId;
use messaging_domain::search::{
    SearchCursor, SearchHit, SearchKind, SearchRepository, clamp_search_limit, normalize_search,
};
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum GlobalSearchError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Unknown search kind")]
    InvalidKind,
    #[error("Search query must be 2 to 100 characters")]
    InvalidQuery,
    #[error("Invalid pagination cursor")]
    InvalidCursor,
}

pub struct GlobalSearchQuery {
    pub query: String,
    pub kind: Option<String>,
    pub before: Option<String>,
    pub limit: Option<i64>,
}

pub struct GlobalSearchPage {
    pub items: Vec<SearchHit>,
    pub next_cursor: Option<String>,
}

pub struct GlobalSearch<R: SearchRepository> {
    repository: Arc<R>,
}

impl<R: SearchRepository> GlobalSearch<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        viewer: UserId,
        query: GlobalSearchQuery,
    ) -> Result<GlobalSearchPage, GlobalSearchError> {
        let kind = match query.kind.as_deref() {
            None => SearchKind::Messages,
            Some(raw) => SearchKind::parse(raw).ok_or(GlobalSearchError::InvalidKind)?,
        };
        let text = normalize_search(&query.query).ok_or(GlobalSearchError::InvalidQuery)?;
        let before = query
            .before
            .as_deref()
            .map(|raw| SearchCursor::decode(raw, kind).ok_or(GlobalSearchError::InvalidCursor))
            .transpose()?;
        let limit = clamp_search_limit(query.limit);
        let mut items = self
            .repository
            .search(&viewer, kind, &text, before.as_ref(), limit + 1)
            .await?;
        let has_more = items.len() as i64 > limit;
        items.truncate(limit as usize);
        let next_cursor = if has_more {
            items.last().map(|item| item.cursor.encode())
        } else {
            None
        };
        Ok(GlobalSearchPage { items, next_cursor })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::{TimeZone, Utc};
    use messaging_domain::search::{SearchItemKey, SearchMatch, SearchPlace};
    use std::sync::Mutex;
    use uuid::Uuid;

    struct FakeRepository {
        calls: Mutex<Vec<(SearchKind, String, Option<SearchCursor>, i64)>>,
        hits: usize,
    }

    #[async_trait]
    impl SearchRepository for FakeRepository {
        async fn search(
            &self,
            _viewer: &UserId,
            kind: SearchKind,
            query: &str,
            before: Option<&SearchCursor>,
            limit: i64,
        ) -> Result<Vec<SearchHit>, RepositoryError> {
            self.calls
                .lock()
                .unwrap()
                .push((kind, query.to_string(), before.copied(), limit));
            Ok((0..self.hits)
                .map(|i| {
                    let created_at = Utc.timestamp_micros(1_000_000 - i as i64).single().unwrap();
                    let message_id = Uuid::from_u128(i as u128 + 1);
                    SearchHit {
                        message_id,
                        author_id: UserId::new(Uuid::from_u128(99)),
                        author_name: "a".into(),
                        forwarded_author_id: None,
                        created_at,
                        place: SearchPlace::Dm {
                            peer_id: Uuid::from_u128(99),
                            peer_name: "a".into(),
                        },
                        snippet: "hello".into(),
                        found: SearchMatch::Message,
                        cursor: SearchCursor {
                            created_at,
                            message_id,
                            item: SearchItemKey::Message,
                        },
                    }
                })
                .collect())
        }
    }

    fn usecase(hits: usize) -> (GlobalSearch<FakeRepository>, Arc<FakeRepository>) {
        let repository = Arc::new(FakeRepository {
            calls: Mutex::new(Vec::new()),
            hits,
        });
        (GlobalSearch::new(repository.clone()), repository)
    }

    fn query(text: &str, kind: Option<&str>, limit: Option<i64>) -> GlobalSearchQuery {
        GlobalSearchQuery {
            query: text.into(),
            kind: kind.map(Into::into),
            before: None,
            limit,
        }
    }

    #[tokio::test]
    async fn fetches_one_extra_hit_to_build_the_next_cursor() {
        let (search, repository) = usecase(3);
        let page = search
            .execute(UserId::new(Uuid::from_u128(1)), query("  hello ", None, Some(2)))
            .await
            .unwrap();
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.next_cursor, Some(page.items[1].cursor.encode()));
        let calls = repository.calls.lock().unwrap();
        assert_eq!(calls[0].0, SearchKind::Messages);
        assert_eq!(calls[0].1, "hello");
        assert_eq!(calls[0].3, 3);
    }

    #[tokio::test]
    async fn last_page_has_no_cursor() {
        let (search, _) = usecase(2);
        let page = search
            .execute(UserId::new(Uuid::from_u128(1)), query("hello", Some("files"), Some(2)))
            .await
            .unwrap();
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.next_cursor, None);
    }

    #[tokio::test]
    async fn rejects_bad_input_before_querying() {
        let (search, repository) = usecase(1);
        let viewer = UserId::new(Uuid::from_u128(1));
        assert!(matches!(
            search.execute(viewer, query("x", None, None)).await,
            Err(GlobalSearchError::InvalidQuery)
        ));
        assert!(matches!(
            search.execute(viewer, query("hello", Some("voice"), None)).await,
            Err(GlobalSearchError::InvalidKind)
        ));
        let mut bad_cursor = query("hello", Some("links"), None);
        bad_cursor.before = Some("1_00000000000000000000000000000001_0".into());
        assert!(matches!(
            search.execute(viewer, bad_cursor).await,
            Err(GlobalSearchError::InvalidCursor)
        ));
        assert!(repository.calls.lock().unwrap().is_empty());
    }
}
