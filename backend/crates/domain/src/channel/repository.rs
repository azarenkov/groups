use async_trait::async_trait;

use crate::{
    channel::{
        Channel,
        id::ChannelId, name::ChannelName,
    },
    server::id::ServerId,
    shared::repository::error::RepositoryError,
    user::id::UserId,
};

#[async_trait]
pub trait ChannelRepository {
    async fn save_channel(&self, channel: &Channel) -> Result<(), RepositoryError>;
    async fn get_channel_name_by_id(&self, channel_id: &ChannelId) -> Result<ChannelName, RepositoryError>;
    async fn get_channel_by_id(&self, channel_id: &ChannelId) -> Result<Channel, RepositoryError>;
    async fn channel_exists(&self, channel_id: &ChannelId) -> Result<bool, RepositoryError>;
    async fn channel_exists_with_server_id(
        &self,
        channel_id: &ChannelId,
        server_id: &ServerId,
    ) -> Result<bool, RepositoryError>;

    async fn delete_channel_by_owner(
        &self,
        channel_id: &ChannelId,
        owner_id: &UserId,
    ) -> Result<bool, RepositoryError>;
}
