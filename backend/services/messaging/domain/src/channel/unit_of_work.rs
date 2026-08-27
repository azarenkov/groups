use domain::{shared::outbox::outbox_record::OutboxRecord, shared::repository::error::RepositoryError, user::id::UserId};

use crate::channel::id::ChannelId;

#[async_trait::async_trait]
pub trait ChannelUnitOfWork: Send + Sync {
    async fn begin(&self) -> Result<Box<dyn ChannelTransaction>, RepositoryError>;
}

#[async_trait::async_trait]
pub trait ChannelTransaction: Send {
    async fn join_text(&mut self, channel_id: &ChannelId, user_id: &UserId) -> Result<bool, RepositoryError>;
    async fn save_outbox(&mut self, records: &[OutboxRecord]) -> Result<(), RepositoryError>;
    async fn commit(self: Box<Self>) -> Result<(), RepositoryError>;
}
