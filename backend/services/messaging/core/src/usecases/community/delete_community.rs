use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{events::MessagingEvent, channel::repository::ChannelRepository, community::events::CommunityEvent, community::events::lifecycle::CommunityDeleted, community::repository::CommunityRepository, community::unit_of_work::CommunityUnitOfWork};
use ports::repository::RepositoryError;

use crate::{
    dto::requests::community::delete_community::DeleteCommunityRequest,
    ports::voice_service::{VoiceService, VoiceServiceError},
};

#[derive(Debug, thiserror::Error)]
pub enum DeleteCommunityError {
    #[error("Community not found")]
    CommunityNotFound,
    #[error("Only the community owner can delete it")]
    NotOwner,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Voice service error: {0}")]
    VoiceService(#[from] VoiceServiceError),
}

pub struct DeleteCommunity<S, C, W, V>
where
    S: CommunityRepository,
    C: ChannelRepository,
    W: CommunityUnitOfWork,
    V: VoiceService,
{
    community_repository: Arc<S>,
    channel_repository: Arc<C>,
    unit_of_work: Arc<W>,
    voice_service: Arc<V>,
}

impl<S, C, W, V> DeleteCommunity<S, C, W, V>
where
    S: CommunityRepository,
    C: ChannelRepository,
    W: CommunityUnitOfWork,
    V: VoiceService,
{
    pub fn new(
        community_repository: Arc<S>,
        channel_repository: Arc<C>,
        unit_of_work: Arc<W>,
        voice_service: Arc<V>,
    ) -> Self {
        Self {
            community_repository,
            channel_repository,
            unit_of_work,
            voice_service,
        }
    }

    pub async fn execute(&self, request: DeleteCommunityRequest) -> Result<bool, DeleteCommunityError> {
        let DeleteCommunityRequest {
            community_id,
            owner_id,
            ctx,
        } = request;

        let actual_owner = self
            .community_repository
            .find_community_owner(&community_id)
            .await?
            .ok_or(DeleteCommunityError::CommunityNotFound)?;

        if actual_owner.uuid() != owner_id.uuid() {
            return Err(DeleteCommunityError::NotOwner);
        }

        let voice_channel_ids = self
            .channel_repository
            .list_voice_channel_ids_by_community(&community_id)
            .await?;

        let member_ids = self
            .community_repository
            .find_member_ids_by_community_id(&community_id)
            .await?;

        let event = CommunityEvent::Deleted(CommunityDeleted::record(
            community_id,
            owner_id,
            member_ids,
            Utc::now(),
        ));
        let events = vec![MessagingEvent::from(event)];

        let mut tx = self.unit_of_work.begin(&community_id).await?;
        if tx.delete_community_by_owner(&owner_id).await?.is_none() {
            return Ok(false);
        }
        tx.save_events(&events, ctx.request_id.as_deref()).await?;
        tx.commit().await?;

        for channel_id in &voice_channel_ids {
            if let Err(err) = self.voice_service.delete_voice_channel(channel_id).await {
                tracing::warn!(
                    channel_id = %channel_id.uuid(),
                    community_id = %community_id.uuid(),
                    error = %err,
                    "Failed to delete LiveKit room after deleting community",
                );
            }
        }

        Ok(true)
    }
}
