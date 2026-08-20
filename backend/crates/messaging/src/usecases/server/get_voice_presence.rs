use std::sync::Arc;

use domain::{
    channel::repository::{ChannelRepository, StreamViewerCountRow, VoicePresenceRow},
    server::{id::ServerId, repository::ServerRepository},
    shared::repository::error::RepositoryError,
    user::id::UserId,
};

pub struct VoicePresenceSnapshot {
    pub presence: Vec<VoicePresenceRow>,
    pub stream_viewers: Vec<StreamViewerCountRow>,
}

#[derive(Debug, thiserror::Error)]
pub enum GetVoicePresenceError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Not a member of this server")]
    NotMember,
}

pub struct GetVoicePresence<S, C>
where
    S: ServerRepository,
    C: ChannelRepository,
{
    server_repository: Arc<S>,
    channel_repository: Arc<C>,
}

impl<S, C> GetVoicePresence<S, C>
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
        requester_id: UserId,
    ) -> Result<VoicePresenceSnapshot, GetVoicePresenceError> {
        if !self
            .server_repository
            .is_server_member(&server_id, &requester_id)
            .await?
        {
            return Err(GetVoicePresenceError::NotMember);
        }

        let presence = self
            .channel_repository
            .list_voice_presence_for_server(&server_id)
            .await?;
        let stream_viewers = self
            .channel_repository
            .list_stream_viewer_counts_for_server(&server_id)
            .await?;

        Ok(VoicePresenceSnapshot {
            presence,
            stream_viewers,
        })
    }
}
