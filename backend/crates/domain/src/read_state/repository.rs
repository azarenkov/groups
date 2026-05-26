use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    channel::id::ChannelId, shared::repository::error::RepositoryError, user::id::UserId,
};

#[derive(Debug, Clone)]
pub struct ChannelUnread {
    pub channel_id: Uuid,
    pub server_id: Uuid,
    pub count: i64,
}

#[derive(Debug, Clone)]
pub struct DmUnread {
    pub friend_id: Uuid,
    pub count: i64,
}

#[async_trait]
pub trait ReadStateRepository {
    async fn mark_channel_read(
        &self,
        user_id: &UserId,
        channel_id: &ChannelId,
    ) -> Result<(), RepositoryError>;

    async fn mark_dm_read(
        &self,
        user_id: &UserId,
        friend_id: &UserId,
    ) -> Result<(), RepositoryError>;

    async fn channel_unread_counts(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<ChannelUnread>, RepositoryError>;

    async fn dm_unread_counts(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<DmUnread>, RepositoryError>;
}
