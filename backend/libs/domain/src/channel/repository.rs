use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::{
    channel::{
        Channel, ChannelError, VoiceChange,
        events::participant::{ConnectionAbortReason, LeaveReason, MediaSource},
        id::ChannelId,
        session_id::SessionId,
    },
    shared::{
        outbox::outbox_record::OutboxRecord,
        pagination::{PaginatedResult, Pagination},
        repository::error::RepositoryError,
    },
    user::{id::UserId, name::UserName},
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
        delegated_actor: Option<&UserId>,
        events: &[OutboxRecord],
    ) -> Result<bool, RepositoryError>;

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
        request_id: Option<String>,
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
        request_id: Option<String>,
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
        request_id: Option<String>,
    ) -> Result<bool, ChannelOperationError>;

    async fn apply_track_published(
        &self,
        channel_id: &ChannelId,
        user_id: UserId,
        session_id: SessionId,
        source: MediaSource,
        occurred_at: DateTime<Utc>,
        inbox_source: &str,
        inbox_external_id: &str,
        request_id: Option<String>,
    ) -> Result<bool, ChannelOperationError>;

    async fn apply_track_unpublished(
        &self,
        channel_id: &ChannelId,
        user_id: UserId,
        session_id: SessionId,
        source: MediaSource,
        occurred_at: DateTime<Utc>,
        inbox_source: &str,
        inbox_external_id: &str,
        request_id: Option<String>,
    ) -> Result<bool, ChannelOperationError>;

    async fn find_channel_by_id(&self, channel_id: &ChannelId) -> Result<Channel, RepositoryError>;

    async fn mute_participant_with_events(
        &self,
        channel_id: &ChannelId,
        target: &UserId,
        muted_by: &UserId,
        muted_at: DateTime<Utc>,
        self_muted: bool,
        server_id: &crate::server::id::ServerId,
        delegated: bool,
        events: &[OutboxRecord],
    ) -> Result<bool, RepositoryError>;

    async fn unmute_participant_with_events(
        &self,
        channel_id: &ChannelId,
        target: &UserId,
        expected_muted_by: Option<&UserId>,
        downgrade_to_self_mute: bool,
        server_id: &crate::server::id::ServerId,
        initiator: &UserId,
        delegated: bool,
        events: &[OutboxRecord],
    ) -> Result<bool, RepositoryError>;

    async fn delete_channel_with_events(
        &self,
        channel_id: &ChannelId,
        actor: &UserId,
        delegated: bool,
        events: &[OutboxRecord],
    ) -> Result<Option<Vec<String>>, RepositoryError>;

    async fn list_voice_channel_ids_by_server(
        &self,
        server_id: &crate::server::id::ServerId,
    ) -> Result<Vec<ChannelId>, RepositoryError>;

    async fn list_channel_participants(
        &self,
        server_id: &crate::server::id::ServerId,
        channel_id: &ChannelId,
        pagination: Pagination,
    ) -> Result<PaginatedResult<(UserId, UserName)>, RepositoryError>;

    async fn join_text_with_events(
        &self,
        channel: &Channel,
        joined_user_id: &UserId,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError>;

    async fn is_text_channel_member(
        &self,
        channel_id: &ChannelId,
        user_id: &UserId,
    ) -> Result<bool, RepositoryError>;

    async fn list_text_channel_member_ids(
        &self,
        channel_id: &ChannelId,
    ) -> Result<Vec<UserId>, RepositoryError>;

    async fn text_members_by_names(
        &self,
        channel_id: &ChannelId,
        lowercase_names: &[String],
    ) -> Result<Vec<(UserId, String)>, RepositoryError>;

    async fn list_voice_presence_for_server(
        &self,
        server_id: &crate::server::id::ServerId,
    ) -> Result<Vec<VoicePresenceRow>, RepositoryError>;

    async fn set_stream_viewer(
        &self,
        server_id: &crate::server::id::ServerId,
        channel_id: &ChannelId,
        streamer_id: &UserId,
        viewer_id: &UserId,
        viewer_session_id: &SessionId,
        watching: bool,
        occurred_at: DateTime<Utc>,
        request_id: Option<String>,
    ) -> Result<i64, RepositoryError>;

    async fn list_stream_viewer_counts_for_server(
        &self,
        server_id: &crate::server::id::ServerId,
    ) -> Result<Vec<StreamViewerCountRow>, RepositoryError>;
}

#[derive(Debug, Clone)]
pub struct StreamViewerCountRow {
    pub channel_id: ChannelId,
    pub streamer_id: UserId,
    pub count: i64,
}

#[derive(Debug, Clone)]
pub struct VoicePresenceRow {
    pub channel_id: ChannelId,
    pub user_id: UserId,
    pub user_name: String,
    pub is_muted: bool,
    pub muted_by: Option<UserId>,
    pub camera_on: bool,
    pub screen_on: bool,
}
