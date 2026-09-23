use std::{sync::Arc, time::Duration};

use domain::user::id::UserId;
use futures::{StreamExt, stream};
use messaging_domain::{channel::repository::ChannelRepository, community::id::CommunityId, community::unit_of_work::CommunityUnitOfWork};
use ports::voice_service::ParticipantAccess;

use crate::usecases::channel::voice_access::participant_access;
use ports::repository::RepositoryError;

use crate::ports::voice_service::{VoiceService, VoiceServiceError};

#[derive(Debug, thiserror::Error)]
pub enum EnforceCommunityDepartureError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Voice service error: {0}")]
    VoiceService(#[from] VoiceServiceError),
    #[error("Voice service did not respond within {0:?}")]
    Timeout(Duration),
}

const LOCKED_ENFORCEMENT_BUDGET: Duration = Duration::from_secs(5);
const MAX_CONCURRENT_REMOVALS: usize = 8;

pub struct EnforceCommunityDeparture<W, C, V>
where
    W: CommunityUnitOfWork,
    C: ChannelRepository,
    V: VoiceService + Send + Sync + 'static,
{
    unit_of_work: Arc<W>,
    channel_repository: Arc<C>,
    voice_service: Arc<V>,
}

impl<W, C, V> EnforceCommunityDeparture<W, C, V>
where
    W: CommunityUnitOfWork,
    C: ChannelRepository,
    V: VoiceService + Send + Sync + 'static,
{
    pub fn new(unit_of_work: Arc<W>, channel_repository: Arc<C>, voice_service: Arc<V>) -> Self {
        Self {
            unit_of_work,
            channel_repository,
            voice_service,
        }
    }

    pub async fn execute(&self, community_id: CommunityId, user_id: UserId) -> Result<(), EnforceCommunityDepartureError> {
        let voice_channel_ids = self
            .channel_repository
            .list_voice_channel_ids_by_community(&community_id)
            .await?;
        let mut tx = self.unit_of_work.begin(&community_id).await?;
        let community = match tx.load_community().await {
            Ok(community) => community,
            Err(RepositoryError::NotFound) => return Ok(()),
            Err(e) => return Err(e.into()),
        };
        let mut actions = Vec::with_capacity(voice_channel_ids.len());
        if community.is_member(&user_id) {
            for channel_id in voice_channel_ids {
                let action = match tx.load_channel(&channel_id).await {
                    Ok(channel) => VoiceAction::Access(participant_access(&channel, &user_id)),
                    Err(RepositoryError::NotFound) => continue,
                    Err(e) => return Err(e.into()),
                };
                actions.push((channel_id, action));
            }
        } else {
            actions.extend(voice_channel_ids.into_iter().map(|c| (c, VoiceAction::Remove)));
        }
        let effects = stream::iter(actions).map(|(channel_id, action)| {
            let voice_service = self.voice_service.clone();
            async move {
                let call = tokio::spawn(async move {
                    match action {
                        VoiceAction::Remove => voice_service.remove_participant(&channel_id, &user_id).await,
                        VoiceAction::Access(access) => {
                            voice_service.apply_participant_access(&channel_id, &user_id, access).await
                        }
                    }
                });
                let abort = call.abort_handle();
                let result = match tokio::time::timeout(LOCKED_ENFORCEMENT_BUDGET, call).await {
                    Ok(Ok(Ok(()))) => Ok(()),
                    Ok(Ok(Err(e))) => Err(EnforceCommunityDepartureError::VoiceService(e)),
                    Ok(Err(join)) => Err(EnforceCommunityDepartureError::VoiceService(
                        VoiceServiceError::ParticipantRemoval(join.to_string()),
                    )),
                    Err(_) => {
                        abort.abort();
                        Err(EnforceCommunityDepartureError::Timeout(LOCKED_ENFORCEMENT_BUDGET))
                    }
                };
                if let Err(e) = &result {
                    tracing::warn!(
                        channel_id = %channel_id.uuid(),
                        user_id = %user_id.uuid(),
                        ?action,
                        error = %e,
                        "Failed to apply voice effect after community departure",
                    );
                }
                result
            }
        });
        let results: Vec<Result<(), EnforceCommunityDepartureError>> = effects
            .buffer_unordered(MAX_CONCURRENT_REMOVALS)
            .collect()
            .await;
        drop(tx);
        results.into_iter().find(Result::is_err).unwrap_or(Ok(()))
    }
}

#[derive(Debug, Clone, Copy)]
enum VoiceAction {
    Remove,
    Access(ParticipantAccess),
}
