use async_trait::async_trait;

use crate::{
    channel::{channel::Channel, id::ChannelId, name::ChannelName},
    server::id::ServerId,
    shared::{outbox::outbox_record::OutboxRecord, repository::error::RepositoryError},
};

#[async_trait]
pub trait ChannelRepository {
    async fn save_channel_with_events(
        &self,
        channel: &Channel,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError>;
    async fn find_channel_name_by_id(
        &self,
        channel_id: &ChannelId,
    ) -> Result<ChannelName, RepositoryError>;

    async fn update_channel_with_events(
        &self,
        channel: &Channel,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError>;

    async fn find_channel_by_id(&self, channel_id: &ChannelId) -> Result<Channel, RepositoryError>;
    async fn channel_exists(&self, channel_id: &ChannelId) -> Result<bool, RepositoryError>;
    async fn channel_exists_with_server_id(
        &self,
        channel_id: &ChannelId,
        server_id: &ServerId,
    ) -> Result<bool, RepositoryError>;

    async fn delete_channel_with_events(
        &self,
        channel_id: &ChannelId,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError>;
}
