use std::sync::Arc;

use domain::user::id::UserId;
use messaging_domain::{channel::id::ChannelId, channel::repository::ChannelRepository};
use ports::repository::RepositoryError;

use crate::ports::voice_service::{VoiceService, VoiceServiceError};

#[derive(Debug, thiserror::Error)]
pub enum EnforceVoiceMuteError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Voice service error: {0}")]
    VoiceService(#[from] VoiceServiceError),
}

pub struct EnforceVoiceMute<C, V>
where
    C: ChannelRepository,
    V: VoiceService,
{
    channel_repository: Arc<C>,
    voice_service: Arc<V>,
}

impl<C, V> EnforceVoiceMute<C, V>
where
    C: ChannelRepository,
    V: VoiceService,
{
    pub fn new(channel_repository: Arc<C>, voice_service: Arc<V>) -> Self {
        Self {
            channel_repository,
            voice_service,
        }
    }

    pub async fn execute(&self, channel_id: ChannelId, user_id: UserId) -> Result<(), EnforceVoiceMuteError> {
        let channel = match self.channel_repository.find_channel_by_id(&channel_id).await {
            Ok(channel) => channel,
            Err(RepositoryError::NotFound) => return Ok(()),
            Err(e) => return Err(e.into()),
        };
        let moderator_muted = channel
            .muted_participants_list()
            .find(|m| m.user_id == user_id)
            .is_some_and(|m| m.muted_by != user_id);
        if moderator_muted {
            self.voice_service.mute_participant(&channel_id, &user_id).await?;
        } else {
            self.voice_service.unmute_participant(&channel_id, &user_id).await?;
        }
        Ok(())
    }
}
