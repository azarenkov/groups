use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::channel::id::ChannelId;
use domain::user::id::UserId;
use ports::repository::RepositoryError;

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

#[derive(Debug, Clone)]
pub struct MemberRead {
    pub user_id: Uuid,
    pub read_at: Option<DateTime<Utc>>,
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
    ) -> Result<bool, RepositoryError>;

    async fn channel_unread_counts(
        &self,
        user_id: &UserId,
        default_level: &str,
    ) -> Result<Vec<ChannelUnread>, RepositoryError>;

    async fn dm_unread_counts(
        &self,
        user_id: &UserId,
        default_level: &str,
    ) -> Result<Vec<DmUnread>, RepositoryError>;

    async fn dm_peer_read_at(
        &self,
        user_id: &UserId,
        friend_id: &UserId,
    ) -> Result<Option<DateTime<Utc>>, RepositoryError>;

    async fn channel_member_read_ats(
        &self,
        channel_id: &ChannelId,
        exclude: &UserId,
    ) -> Result<Vec<MemberRead>, RepositoryError>;
}
