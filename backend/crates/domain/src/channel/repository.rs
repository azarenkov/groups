use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::{
    channel::{Channel, id::ChannelId},
    shared::{outbox::outbox_record::OutboxRecord, repository::error::RepositoryError},
    user::id::UserId,
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

    async fn record_events(
        &self,
        channel_id: &ChannelId,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError>;

    async fn record_events_with_inbox(
        &self,
        channel_id: &ChannelId,
        inbox_source: &str,
        inbox_external_id: &str,
        events: &[OutboxRecord],
    ) -> Result<bool, RepositoryError>;

    async fn find_channel_by_id(&self, channel_id: &ChannelId) -> Result<Channel, RepositoryError>;

    async fn mute_participant_with_events(
        &self,
        channel_id: &ChannelId,
        target: &UserId,
        muted_by: &UserId,
        muted_at: DateTime<Utc>,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError>;

    async fn unmute_participant_with_events(
        &self,
        channel_id: &ChannelId,
        target: &UserId,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError>;

    async fn delete_channel_with_events(
        &self,
        channel_id: &ChannelId,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError>;
}
