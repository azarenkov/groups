use async_trait::async_trait;

use crate::{
    dm_message::{DmMessage, id::DmMessageId, view::DmMessageView},
    server::id::ServerId,
    shared::{
        outbox::outbox_record::OutboxRecord,
        pagination::{PaginatedResult, Pagination},
        repository::error::RepositoryError,
    },
    user::id::UserId,
};

#[async_trait]
pub trait DmMessageRepository {
    async fn save_with_events(
        &self,
        message: &DmMessage,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError>;

    async fn update_with_events(
        &self,
        message: &DmMessage,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError>;

    async fn delete_with_events(
        &self,
        id: &DmMessageId,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError>;

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

    async fn has_pending_server_invite(
        &self,
        sender_id: &UserId,
        recipient_id: &UserId,
        server_id: &ServerId,
    ) -> Result<bool, RepositoryError>;
}
