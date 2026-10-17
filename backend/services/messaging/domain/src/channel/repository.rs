use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::{channel::Channel, channel::id::ChannelId, channel::session_id::SessionId};
use domain::{pagination::PaginatedResult, pagination::Pagination, user::id::UserId, user::name::UserName};
use ports::repository::RepositoryError;

#[async_trait]
#[allow(clippy::too_many_arguments)]
pub trait ChannelRepository {

    async fn find_channel_by_id(&self, channel_id: &ChannelId) -> Result<Channel, RepositoryError>;

    async fn list_voice_channel_ids_by_community(
        &self,
        community_id: &crate::community::id::CommunityId,
    ) -> Result<Vec<ChannelId>, RepositoryError>;
    async fn voice_channel_ids_among(&self, ids: &[ChannelId]) -> Result<Vec<ChannelId>, RepositoryError>;
    async fn find_visible_voice_room(
        &self,
        target: &UserId,
        viewer: &UserId,
    ) -> Result<Option<VisibleVoiceRoom>, RepositoryError>;
    async fn list_voice_channel_ids_with_participant(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<ChannelId>, RepositoryError>;

    async fn list_channel_participants(
        &self,
        community_id: &crate::community::id::CommunityId,
        channel_id: &ChannelId,
        pagination: Pagination,
    ) -> Result<PaginatedResult<(UserId, UserName, String, Option<DateTime<Utc>>)>, RepositoryError>;

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

    async fn list_voice_presence_for_community(
        &self,
        community_id: &crate::community::id::CommunityId,
    ) -> Result<Vec<VoicePresenceRow>, RepositoryError>;

    async fn set_stream_viewer(
        &self,
        community_id: &crate::community::id::CommunityId,
        channel_id: &ChannelId,
        streamer_id: &UserId,
        viewer_id: &UserId,
        viewer_session_id: &SessionId,
        watching: bool,
        occurred_at: DateTime<Utc>,
        request_id: Option<String>,
    ) -> Result<i64, RepositoryError>;

    async fn list_stream_viewer_counts_for_community(
        &self,
        community_id: &crate::community::id::CommunityId,
    ) -> Result<Vec<StreamViewerCountRow>, RepositoryError>;

    async fn list_voice_sessions_for_community(
        &self,
        community_id: &crate::community::id::CommunityId,
    ) -> Result<Vec<VoiceSessionRow>, RepositoryError>;

    async fn list_voice_participants_joined_before(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<Vec<VoiceParticipantRow>, RepositoryError>;

    async fn list_friends_voice_presence(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<FriendVoicePresenceRow>, RepositoryError>;

    async fn list_recent_voice_rooms(
        &self,
        user_id: &UserId,
        limit: i64,
    ) -> Result<Vec<RecentVoiceRoomRow>, RepositoryError>;
}

#[derive(Debug, Clone)]
pub struct VisibleVoiceRoom {
    pub channel_id: ChannelId,
    pub channel_name: String,
    pub community_id: crate::community::id::CommunityId,
    pub community_name: String,
}

#[derive(Debug, Clone)]
pub struct FriendVoicePresenceRow {
    pub user_id: UserId,
    pub user_name: String,
    pub channel_id: ChannelId,
    pub channel_name: String,
    pub community_id: crate::community::id::CommunityId,
    pub community_name: String,
    pub joined_at: DateTime<Utc>,
    pub is_muted: bool,
    pub camera_on: bool,
    pub screen_on: bool,
    pub profile_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct RecentVoiceRoomRow {
    pub channel_id: ChannelId,
    pub channel_name: String,
    pub community_id: crate::community::id::CommunityId,
    pub community_name: String,
    pub live_count: i64,
    pub last_join: DateTime<Utc>,
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
pub struct VoiceSessionRow {
    pub channel_id: ChannelId,
    pub started_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct VoicePresenceRow {
    pub channel_id: ChannelId,
    pub user_id: UserId,
    pub user_name: String,
    pub is_muted: bool,
    pub muted_by: Option<UserId>,
    pub deafened_by: Option<UserId>,
    pub camera_on: bool,
    pub screen_on: bool,
}
