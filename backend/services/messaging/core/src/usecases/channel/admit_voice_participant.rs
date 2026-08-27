use std::sync::Arc;

use domain::{
    channel::{id::ChannelId, repository::ChannelRepository},
    server::repository::ServerRepository,
    shared::repository::error::RepositoryError,
    user::id::UserId,
};

use crate::ports::voice_service::{VoiceService, VoiceServiceError};

#[derive(Debug, thiserror::Error)]
pub enum AdmitVoiceParticipantError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Voice service error: {0}")]
    VoiceService(#[from] VoiceServiceError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    Admitted,
    Rejected,
}

pub struct AdmitVoiceParticipant<C, S, V>
where
    C: ChannelRepository,
    S: ServerRepository,
    V: VoiceService,
{
    channel_repository: Arc<C>,
    server_repository: Arc<S>,
    voice_service: Arc<V>,
}

impl<C, S, V> AdmitVoiceParticipant<C, S, V>
where
    C: ChannelRepository,
    S: ServerRepository,
    V: VoiceService,
{
    pub fn new(channel_repository: Arc<C>, server_repository: Arc<S>, voice_service: Arc<V>) -> Self {
        Self {
            channel_repository,
            server_repository,
            voice_service,
        }
    }

    pub async fn execute(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> Result<Admission, AdmitVoiceParticipantError> {
        let channel = match self.channel_repository.find_channel_by_id(&channel_id).await {
            Ok(channel) => channel,
            Err(RepositoryError::NotFound) => {
                self.voice_service.delete_voice_channel(&channel_id).await?;
                tracing::info!(
                    channel_id = %channel_id.uuid(),
                    user_id = %user_id.uuid(),
                    "Closed orphaned LiveKit room after join into a deleted channel",
                );
                return Ok(Admission::Rejected);
            }
            Err(e) => return Err(e.into()),
        };

        let is_member = self
            .server_repository
            .is_server_member(channel.server_id(), &user_id)
            .await?;
        if !is_member {
            self.voice_service
                .remove_participant(&channel_id, &user_id)
                .await?;
            tracing::info!(
                channel_id = %channel_id.uuid(),
                user_id = %user_id.uuid(),
                "Disconnected non-member voice participant",
            );
            return Ok(Admission::Rejected);
        }

        let moderator_muted = channel
            .muted_participants_list()
            .find(|m| m.user_id == user_id)
            .is_some_and(|m| m.muted_by != user_id);
        if moderator_muted {
            self.voice_service.mute_participant(&channel_id, &user_id).await?;
        } else {
            self.voice_service
                .unmute_participant(&channel_id, &user_id)
                .await?;
        }
        Ok(Admission::Admitted)
    }
}
