use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{channel::id::ChannelId, channel::repository::ChannelRepository, channel::session_id::SessionId, community::id::CommunityId, community::repository::CommunityRepository};
use domain::user::id::UserId;
use ports::repository::RepositoryError;

use crate::dto::requests::channel::update_stream_viewer::UpdateStreamViewerRequest;

#[derive(Debug, thiserror::Error)]
pub enum UpdateStreamViewerError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Not a member of this community")]
    NotMember,
}

pub struct UpdateStreamViewer<S, C>
where
    S: CommunityRepository,
    C: ChannelRepository,
{
    community_repository: Arc<S>,
    channel_repository: Arc<C>,
}

impl<S, C> UpdateStreamViewer<S, C>
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
        viewer_id: UserId,
        request: UpdateStreamViewerRequest,
    ) -> Result<i64, UpdateStreamViewerError> {
        if !self
            .community_repository
            .is_community_member(&community_id, &viewer_id)
            .await?
        {
            return Err(UpdateStreamViewerError::NotMember);
        }

        let channel_id = ChannelId::new(request.channel_id);
        let streamer_id = UserId::new(request.streamer_id);
        let viewer_session_id = SessionId::new(request.viewer_session_id);

        let count = self
            .channel_repository
            .set_stream_viewer(
                &community_id,
                &channel_id,
                &streamer_id,
                &viewer_id,
                &viewer_session_id,
                request.watching,
                Utc::now(),
                request.ctx.request_id,
            )
            .await?;

        Ok(count)
    }
}
