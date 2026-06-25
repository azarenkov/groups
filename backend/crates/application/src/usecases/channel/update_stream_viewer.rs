use std::sync::Arc;

use chrono::Utc;
use domain::{
    channel::{id::ChannelId, repository::ChannelRepository},
    server::{id::ServerId, repository::ServerRepository},
    shared::repository::error::RepositoryError,
    user::id::UserId,
};

use crate::dto::requests::channel::update_stream_viewer::UpdateStreamViewerRequest;

#[derive(Debug, thiserror::Error)]
pub enum UpdateStreamViewerError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Not a member of this server")]
    NotMember,
}

pub struct UpdateStreamViewer<S, C>
where
    S: ServerRepository,
    C: ChannelRepository,
{
    server_repository: Arc<S>,
    channel_repository: Arc<C>,
}

impl<S, C> UpdateStreamViewer<S, C>
where
    S: ServerRepository,
    C: ChannelRepository,
{
    pub fn new(server_repository: Arc<S>, channel_repository: Arc<C>) -> Self {
        Self {
            server_repository,
            channel_repository,
        }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        viewer_id: UserId,
        request: UpdateStreamViewerRequest,
    ) -> Result<i64, UpdateStreamViewerError> {
        if !self
            .server_repository
            .is_server_member(&server_id, &viewer_id)
            .await?
        {
            return Err(UpdateStreamViewerError::NotMember);
        }

        let channel_id = ChannelId::new(request.channel_id);
        let streamer_id = UserId::new(request.streamer_id);

        let count = self
            .channel_repository
            .set_stream_viewer(
                &server_id,
                &channel_id,
                &streamer_id,
                &viewer_id,
                request.watching,
                Utc::now(),
                request.ctx.request_id,
            )
            .await?;

        Ok(count)
    }
}
