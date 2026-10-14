use std::sync::Arc;

use domain::user::id::UserId;
use messaging_domain::people::{PeopleDirectory, PersonMatch, clamp_people_limit, normalize_people_query};
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum SearchPeopleError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("People search query must be 2 to 32 characters")]
    InvalidQuery,
}

pub struct SearchPeople<D: PeopleDirectory> {
    directory: Arc<D>,
}

impl<D: PeopleDirectory> SearchPeople<D> {
    pub fn new(directory: Arc<D>) -> Self {
        Self { directory }
    }

    pub async fn execute(
        &self,
        viewer: UserId,
        query: &str,
        limit: Option<i64>,
    ) -> Result<Vec<PersonMatch>, SearchPeopleError> {
        let prefix = normalize_people_query(query).ok_or(SearchPeopleError::InvalidQuery)?;
        Ok(self.directory.search(&viewer, &prefix, clamp_people_limit(limit)).await?)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use messaging_domain::people::PersonRelation;
    use uuid::Uuid;

    use super::*;

    #[derive(Default)]
    struct Directory(Mutex<Vec<(String, i64)>>);

    #[async_trait]
    impl PeopleDirectory for Directory {
        async fn search(&self, _: &UserId, prefix: &str, limit: i64) -> Result<Vec<PersonMatch>, RepositoryError> {
            self.0.lock().unwrap().push((prefix.to_string(), limit));
            Ok(vec![PersonMatch {
                user_id: UserId::new(Uuid::nil()),
                name: "alex".to_string(),
                relation: PersonRelation::None,
                accepts_friend_requests: true,
            }])
        }
    }

    #[tokio::test]
    async fn searches_by_the_folded_prefix_with_a_clamped_limit() {
        let directory = Arc::new(Directory::default());
        let usecase = SearchPeople::new(directory.clone());
        let found = usecase.execute(UserId::new(Uuid::new_v4()), " AL ", Some(99)).await.unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(*directory.0.lock().unwrap(), vec![("al".to_string(), 20)]);
    }

    #[tokio::test]
    async fn rejects_too_short_queries_without_touching_the_directory() {
        let directory = Arc::new(Directory::default());
        let usecase = SearchPeople::new(directory.clone());
        let err = usecase.execute(UserId::new(Uuid::new_v4()), "a", None).await.unwrap_err();
        assert!(matches!(err, SearchPeopleError::InvalidQuery));
        assert!(directory.0.lock().unwrap().is_empty());
    }
}
