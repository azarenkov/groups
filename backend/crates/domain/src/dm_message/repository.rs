use async_trait::async_trait;

use crate::{
    dm_message::{DmMessage, id::DmMessageId, view::DmMessageView},
    shared::{
        pagination::{PaginatedResult, Pagination},
        repository::error::RepositoryError,
    },
    user::id::UserId,
};

#[async_trait]
pub trait DmMessageRepository {
    async fn save(&self, message: &DmMessage) -> Result<(), RepositoryError>;

    async fn update(&self, message: &DmMessage) -> Result<(), RepositoryError>;

    async fn delete(&self, id: &DmMessageId) -> Result<(), RepositoryError>;

    async fn find_by_id(&self, id: &DmMessageId) -> Result<DmMessage, RepositoryError>;

    async fn list_between(
        &self,
        user_a: &UserId,
        user_b: &UserId,
        before: Option<&DmMessageId>,
        after: Option<&DmMessageId>,
        limit: i64,
    ) -> Result<Vec<DmMessageView>, RepositoryError>;

    async fn search_between(
        &self,
        user_a: &UserId,
        user_b: &UserId,
        query: &str,
        pagination: Pagination,
    ) -> Result<PaginatedResult<DmMessageView>, RepositoryError>;
}
