use std::{sync::Arc, time::Duration};

use domain::user::id::UserId;
use futures::{StreamExt, stream};
use messaging_domain::{channel::repository::ChannelRepository, server::id::ServerId, server::unit_of_work::ServerUnitOfWork};
use ports::repository::RepositoryError;

use crate::ports::voice_service::{VoiceService, VoiceServiceError};

#[derive(Debug, thiserror::Error)]
pub enum EnforceServerDepartureError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Voice service error: {0}")]
    VoiceService(#[from] VoiceServiceError),
    #[error("Voice service did not respond within {0:?}")]
    Timeout(Duration),
}

const LOCKED_ENFORCEMENT_BUDGET: Duration = Duration::from_secs(5);
const MAX_CONCURRENT_REMOVALS: usize = 8;

pub struct EnforceServerDeparture<W, C, V>
where
    W: ServerUnitOfWork,
    C: ChannelRepository,
    V: VoiceService,
{
    unit_of_work: Arc<W>,
    channel_repository: Arc<C>,
    voice_service: Arc<V>,
}

impl<W, C, V> EnforceServerDeparture<W, C, V>
where
    W: ServerUnitOfWork,
    C: ChannelRepository,
    V: VoiceService,
{
    pub fn new(unit_of_work: Arc<W>, channel_repository: Arc<C>, voice_service: Arc<V>) -> Self {
        Self {
            unit_of_work,
            channel_repository,
            voice_service,
        }
    }

    pub async fn execute(&self, server_id: ServerId, user_id: UserId) -> Result<(), EnforceServerDepartureError> {
        let voice_channel_ids = self
            .channel_repository
            .list_voice_channel_ids_by_server(&server_id)
            .await?;
        let mut tx = self.unit_of_work.begin(&server_id).await?;
        let server = match tx.load_server().await {
            Ok(server) => server,
            Err(RepositoryError::NotFound) => return Ok(()),
            Err(e) => return Err(e.into()),
        };
        if server.is_member(&user_id) {
            return Ok(());
        }
        let removals = stream::iter(voice_channel_ids.iter()).map(|channel_id| async move {
            let outcome = tokio::time::timeout(
                LOCKED_ENFORCEMENT_BUDGET,
                self.voice_service.remove_participant(channel_id, &user_id),
            )
            .await;
            let result = match outcome {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => Err(EnforceServerDepartureError::VoiceService(e)),
                Err(_) => Err(EnforceServerDepartureError::Timeout(LOCKED_ENFORCEMENT_BUDGET)),
            };
            if let Err(e) = &result {
                tracing::warn!(
                    channel_id = %channel_id.uuid(),
                    user_id = %user_id.uuid(),
                    error = %e,
                    "Failed to remove departed participant from voice room",
                );
            }
            result
        });
        let results: Vec<Result<(), EnforceServerDepartureError>> = removals
            .buffer_unordered(MAX_CONCURRENT_REMOVALS)
            .collect()
            .await;
        drop(tx);
        results.into_iter().find(Result::is_err).unwrap_or(Ok(()))
    }
}
