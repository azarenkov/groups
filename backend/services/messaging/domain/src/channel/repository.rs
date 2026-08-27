use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::{channel::Channel, channel::ChannelError, channel::events::participant::ConnectionAbortReason, channel::events::participant::LeaveReason, channel::events::participant::MediaSource, channel::id::ChannelId, channel::session_id::SessionId};
use domain::{pagination::PaginatedResult, pagination::Pagination, user::id::UserId, user::name::UserName};
use ports::repository::RepositoryError;

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

    async fn list_voice_participants_joined_before(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<Vec<VoiceParticipantRow>, RepositoryError>;
}

#[derive(Debug, Clone)]
pub struct VoiceParticipantRow {
    pub channel_id: ChannelId,
    pub user_id: UserId,
    pub session_id: String,
    pub camera_on: bool,
    pub screen_on: bool,
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
