use async_trait::async_trait;

use crate::{
    channel::{channel::Channel, id::ChannelId},
    shared::{outbox::outbox_record::OutboxRecord, repository::error::RepositoryError},
};

#[async_trait]
pub trait ChannelRepository {
    async fn save_channel_with_events(
        &self,
        channel: &Channel,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError>;

    async fn update_channel_with_events(
        &self,
        channel: &Channel,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError>;

    async fn record_events(&self, events: &[OutboxRecord]) -> Result<(), RepositoryError>;

    async fn find_channel_by_id(&self, channel_id: &ChannelId) -> Result<Channel, RepositoryError>;

    async fn delete_channel_with_events(
        &self,
        channel_id: &ChannelId,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError>;
}
