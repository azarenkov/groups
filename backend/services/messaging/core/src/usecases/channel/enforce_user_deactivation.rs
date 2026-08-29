use std::{sync::Arc, time::Duration};

use domain::user::id::UserId;
use futures::{StreamExt, stream};
use messaging_domain::channel::repository::ChannelRepository;
use ports::repository::RepositoryError;

use crate::ports::voice_service::{VoiceService, VoiceServiceError};

#[derive(Debug, thiserror::Error)]
pub enum EnforceUserDeactivationError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Voice service error: {0}")]
    VoiceService(#[from] VoiceServiceError),
    #[error("Voice service did not respond within {0:?}")]
    Timeout(Duration),
}

const REMOVAL_BUDGET: Duration = Duration::from_secs(5);
const MAX_CONCURRENT_REMOVALS: usize = 8;

pub struct EnforceUserDeactivation<C, V>
where
    C: ChannelRepository,
    V: VoiceService + Send + Sync + 'static,
{
    channel_repository: Arc<C>,
    voice_service: Arc<V>,
}

impl<C, V> EnforceUserDeactivation<C, V>
where
    C: ChannelRepository,
    V: VoiceService + Send + Sync + 'static,
{
    pub fn new(channel_repository: Arc<C>, voice_service: Arc<V>) -> Self {
        Self {
            channel_repository,
            voice_service,
        }
    }

    pub async fn execute(&self, user_id: UserId) -> Result<(), EnforceUserDeactivationError> {
        let channel_ids = self
            .channel_repository
            .list_voice_channel_ids_with_participant(&user_id)
            .await?;
        let removals = stream::iter(channel_ids).map(|channel_id| {
            let voice_service = self.voice_service.clone();
            async move {
                let call = tokio::spawn(async move {
                    voice_service.remove_participant(&channel_id, &user_id).await
                });
                let abort = call.abort_handle();
                let result = match tokio::time::timeout(REMOVAL_BUDGET, call).await {
                    Ok(Ok(Ok(()))) => Ok(()),
                    Ok(Ok(Err(e))) => Err(EnforceUserDeactivationError::VoiceService(e)),
                    Ok(Err(join)) => Err(EnforceUserDeactivationError::VoiceService(
                        VoiceServiceError::ParticipantRemoval(join.to_string()),
                    )),
                    Err(_) => {
                        abort.abort();
                        Err(EnforceUserDeactivationError::Timeout(REMOVAL_BUDGET))
                    }
                };
                if let Err(e) = &result {
                    tracing::warn!(
                        channel_id = %channel_id.uuid(),
                        user_id = %user_id.uuid(),
                        error = %e,
                        "Failed to remove deactivated user from voice channel",
                    );
                }
                result
            }
        });
        let results: Vec<Result<(), EnforceUserDeactivationError>> = removals
            .buffer_unordered(MAX_CONCURRENT_REMOVALS)
            .collect()
            .await;
        results.into_iter().find(Result::is_err).unwrap_or(Ok(()))
    }
}
