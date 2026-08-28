use std::{sync::Arc, time::Duration};

use domain::user::id::UserId;
use messaging_domain::{channel::id::ChannelId, channel::moderation::Publication, channel::repository::ChannelRepository, server::unit_of_work::ServerUnitOfWork};
use ports::repository::RepositoryError;

use crate::ports::voice_service::{VoiceService, VoiceServiceError};

#[derive(Debug, thiserror::Error)]
pub enum EnforceVoiceMuteError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Voice service error: {0}")]
    VoiceService(#[from] VoiceServiceError),
    #[error("Voice service did not respond within {0:?}")]
    Timeout(Duration),
}

const LOCKED_ENFORCEMENT_BUDGET: Duration = Duration::from_secs(5);

pub struct EnforceVoiceMute<C, W, V>
where
    C: ChannelRepository,
    W: ServerUnitOfWork,
    V: VoiceService,
{
    channel_repository: Arc<C>,
    unit_of_work: Arc<W>,
    voice_service: Arc<V>,
}

impl<C, W, V> EnforceVoiceMute<C, W, V>
where
    C: ChannelRepository,
    W: ServerUnitOfWork,
    V: VoiceService,
{
    pub fn new(channel_repository: Arc<C>, unit_of_work: Arc<W>, voice_service: Arc<V>) -> Self {
        Self {
            channel_repository,
            unit_of_work,
            voice_service,
        }
    }

    pub async fn execute(&self, channel_id: ChannelId, user_id: UserId) -> Result<(), EnforceVoiceMuteError> {
        let server_id = match self.channel_repository.find_channel_by_id(&channel_id).await {
            Ok(channel) => *channel.server_id(),
            Err(RepositoryError::NotFound) => return Ok(()),
            Err(e) => return Err(e.into()),
        };
        let mut tx = self.unit_of_work.begin(&server_id).await?;
        let server = match tx.load_server().await {
            Ok(server) => server,
            Err(RepositoryError::NotFound) => return Ok(()),
            Err(e) => return Err(e.into()),
        };
        let is_member = server.is_member(&user_id);
        let publication = if is_member {
            match tx.load_channel(&channel_id).await {
                Ok(channel) => Some(channel.publication_for(&user_id)),
                Err(RepositoryError::NotFound) => return Ok(()),
                Err(e) => return Err(e.into()),
            }
        } else {
            None
        };
        let enforcement = async {
            match publication {
                None => self.voice_service.remove_participant(&channel_id, &user_id).await,
                Some(Publication::Restricted) => self.voice_service.mute_participant(&channel_id, &user_id).await,
                Some(Publication::Full) => self.voice_service.unmute_participant(&channel_id, &user_id).await,
            }
        };
        let outcome = tokio::time::timeout(LOCKED_ENFORCEMENT_BUDGET, enforcement).await;
        drop(tx);
        match outcome {
            Ok(result) => result.map_err(EnforceVoiceMuteError::from),
            Err(_) => Err(EnforceVoiceMuteError::Timeout(LOCKED_ENFORCEMENT_BUDGET)),
        }
    }
}
