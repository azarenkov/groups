use std::sync::Arc;

use domain::user::id::UserId;
use messaging_domain::{channel::repository::ChannelRepository, server::id::ServerId, server::repository::ServerRepository};
use ports::repository::RepositoryError;

use crate::ports::voice_service::{VoiceService, VoiceServiceError};

#[derive(Debug, thiserror::Error)]
pub enum EnforceServerDepartureError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Voice service error: {0}")]
    VoiceService(#[from] VoiceServiceError),
}

pub struct EnforceServerDeparture<S, C, V>
where
    S: ServerRepository,
    C: ChannelRepository,
    V: VoiceService,
{
    server_repository: Arc<S>,
    channel_repository: Arc<C>,
    voice_service: Arc<V>,
}

impl<S, C, V> EnforceServerDeparture<S, C, V>
where
    S: ServerRepository,
    C: ChannelRepository,
    V: VoiceService,
{
    pub fn new(server_repository: Arc<S>, channel_repository: Arc<C>, voice_service: Arc<V>) -> Self {
        Self {
            server_repository,
            channel_repository,
            voice_service,
        }
    }

    pub async fn execute(&self, server_id: ServerId, user_id: UserId) -> Result<(), EnforceServerDepartureError> {
        if self.server_repository.is_server_member(&server_id, &user_id).await? {
            return Ok(());
        }
        for channel_id in self
            .channel_repository
            .list_voice_channel_ids_by_server(&server_id)
            .await?
        {
            self.voice_service.remove_participant(&channel_id, &user_id).await?;
        }
        Ok(())
    }
}
