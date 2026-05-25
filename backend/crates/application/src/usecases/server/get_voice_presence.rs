use std::sync::Arc;

use domain::{
    channel::repository::{ChannelRepository, VoicePresenceRow},
    server::{id::ServerId, repository::ServerRepository},
    shared::repository::error::RepositoryError,
    user::id::UserId,
};

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
    ) -> Result<Vec<VoicePresenceRow>, GetVoicePresenceError> {
        if !self
            .server_repository
            .is_server_member(&server_id, &requester_id)
            .await?
        {
            return Err(GetVoicePresenceError::NotMember);
        }

        Ok(self
            .channel_repository
            .list_voice_presence_for_server(&server_id)
            .await?)
    }
}
