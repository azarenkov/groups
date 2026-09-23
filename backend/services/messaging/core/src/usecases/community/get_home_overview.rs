use std::sync::Arc;

use domain::user::id::UserId;
use messaging_domain::{channel::repository::ChannelRepository, channel::repository::FriendVoicePresenceRow, channel::repository::RecentVoiceRoomRow};
use ports::repository::RepositoryError;

const RECENT_ROOMS_LIMIT: i64 = 6;

pub struct HomeOverview {
    pub active_friends: Vec<FriendVoicePresenceRow>,
    pub recent_rooms: Vec<RecentVoiceRoomRow>,
}

#[derive(Debug, thiserror::Error)]
pub enum GetHomeOverviewError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct GetHomeOverview<C: ChannelRepository> {
    channel_repository: Arc<C>,
}

impl<C: ChannelRepository> GetHomeOverview<C> {
    pub fn new(channel_repository: Arc<C>) -> Self {
        Self { channel_repository }
    }

    pub async fn execute(&self, user_id: UserId) -> Result<HomeOverview, GetHomeOverviewError> {
        let active_friends = self
            .channel_repository
            .list_friends_voice_presence(&user_id)
            .await?;
        let recent_rooms = self
            .channel_repository
            .list_recent_voice_rooms(&user_id, RECENT_ROOMS_LIMIT)
            .await?;
        Ok(HomeOverview {
            active_friends,
            recent_rooms,
        })
    }
}
