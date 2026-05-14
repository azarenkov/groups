use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::{
    channel::{
        Channel, ChannelError, VoiceChange,
        events::participant::{ConnectionAbortReason, LeaveReason},
        id::ChannelId,
        session_id::SessionId,
    },
    shared::{outbox::outbox_record::OutboxRecord, repository::error::RepositoryError},
    user::id::UserId,
};

#[derive(Debug, thiserror::Error)]
pub enum ChannelOperationError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Channel domain error: {0}")]
    Domain(#[from] ChannelError),
}

#[async_trait]
#[allow(clippy::too_many_arguments)]
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
        voice_changes: &[VoiceChange],
    ) -> Result<(), RepositoryError>;

    async fn apply_participant_joined(
        &self,
        channel_id: &ChannelId,
        user_id: UserId,
        session_id: SessionId,
        occurred_at: DateTime<Utc>,
        inbox_source: &str,
        inbox_external_id: &str,
    ) -> Result<bool, ChannelOperationError>;

    async fn apply_participant_left(
        &self,
        channel_id: &ChannelId,
        user_id: UserId,
        session_id: SessionId,
        reason: LeaveReason,
        occurred_at: DateTime<Utc>,
        inbox_source: &str,
        inbox_external_id: &str,
    ) -> Result<bool, ChannelOperationError>;

    async fn apply_participant_connection_aborted(
        &self,
        channel_id: &ChannelId,
        user_id: UserId,
        session_id: SessionId,
        reason: ConnectionAbortReason,
        occurred_at: DateTime<Utc>,
        inbox_source: &str,
        inbox_external_id: &str,
    ) -> Result<bool, ChannelOperationError>;

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

    async fn list_voice_channel_ids_by_server(
        &self,
        server_id: &crate::server::id::ServerId,
    ) -> Result<Vec<ChannelId>, RepositoryError>;
}
