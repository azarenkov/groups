use std::sync::Arc;

use domain::{
    shared::repository::error::RepositoryError,
    user::{id::UserId, repository::UserRepository},
};
use uuid::Uuid;

const MAX_LIMIT: i64 = 20;
const DEFAULT_LIMIT: i64 = 10;

#[derive(Debug, thiserror::Error)]
pub enum SearchUsersError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Query too short")]
    QueryTooShort,
}

#[derive(Debug)]
pub struct UserSearchResult {
    pub id: Uuid,
    pub name: String,
}

pub struct SearchUsers<U>
where
    U: UserRepository,
{
    user_repository: Arc<U>,
}

impl<U> SearchUsers<U>
where
    U: UserRepository,
{
    pub fn new(user_repository: Arc<U>) -> Self {
        Self { user_repository }
    }

    pub async fn execute(
        &self,
        query: &str,
        limit: Option<i64>,
        requester_id: UserId,
    ) -> Result<Vec<UserSearchResult>, SearchUsersError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Err(SearchUsersError::QueryTooShort);
        }

        let limit = limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);

        let rows = self
            .user_repository
            .search_by_name_prefix(trimmed, limit, &requester_id)
            .await?;

        Ok(rows
            .into_iter()
            .map(|(id, name)| UserSearchResult {
                id: id.uuid(),
                name: name.as_str().to_string(),
            })
            .collect())
    }
}
