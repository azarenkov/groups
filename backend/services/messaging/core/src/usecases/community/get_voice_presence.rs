use std::sync::Arc;

use messaging_domain::{channel::repository::ChannelRepository, channel::repository::StreamViewerCountRow, channel::repository::VoicePresenceRow, community::id::CommunityId, community::repository::CommunityRepository};
use domain::user::id::UserId;
use ports::repository::RepositoryError;

pub struct VoicePresenceSnapshot {
    pub presence: Vec<VoicePresenceRow>,
    pub stream_viewers: Vec<StreamViewerCountRow>,
}

#[derive(Debug, thiserror::Error)]
pub enum GetVoicePresenceError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Not a member of this community")]
    NotMember,
}

pub struct GetVoicePresence<S, C>
where
    S: CommunityRepository,
    C: ChannelRepository,
{
    community_repository: Arc<S>,
    channel_repository: Arc<C>,
}

impl<S, C> GetVoicePresence<S, C>
where
    S: CommunityRepository,
    C: ChannelRepository,
{
    pub fn new(community_repository: Arc<S>, channel_repository: Arc<C>) -> Self {
        Self {
            community_repository,
            channel_repository,
        }
    }

    pub async fn execute(
        &self,
        community_id: CommunityId,
        requester_id: UserId,
    ) -> Result<VoicePresenceSnapshot, GetVoicePresenceError> {
        if !self
            .community_repository
            .is_community_member(&community_id, &requester_id)
            .await?
        {
            return Err(GetVoicePresenceError::NotMember);
        }

        let presence = self
            .channel_repository
            .list_voice_presence_for_community(&community_id)
            .await?;
        let stream_viewers = self
            .channel_repository
            .list_stream_viewer_counts_for_community(&community_id)
            .await?;

        Ok(VoicePresenceSnapshot {
            presence,
            stream_viewers,
        })
    }
}
