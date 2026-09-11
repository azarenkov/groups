use std::sync::Arc;

use chrono::Utc;
use domain::{channel::id::ChannelId, user::id::UserId};
use ports::repository::RepositoryError;
use uuid::Uuid;
use voice_domain::call::repository::CallRepository;

use crate::ports::{
    friendship_lookup::{FriendshipLookup, FriendshipLookupError},
    voice_service::{VoiceService, VoiceServiceError},
};

#[derive(Debug, thiserror::Error)]
pub enum EnforceBlockError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Voice service error: {0}")]
    Voice(#[from] VoiceServiceError),
    #[error("Friendship lookup unavailable: {0}")]
    FriendshipLookup(#[from] FriendshipLookupError),
}

pub struct EnforceBlock<C, V, F>
where
    C: CallRepository,
    V: VoiceService,
    F: FriendshipLookup,
{
    call_repository: Arc<C>,
    voice_service: Arc<V>,
    friendship_repository: Arc<F>,
}

impl<C, V, F> EnforceBlock<C, V, F>
where
    C: CallRepository,
    V: VoiceService,
    F: FriendshipLookup,
{
    pub fn new(call_repository: Arc<C>, voice_service: Arc<V>, friendship_repository: Arc<F>) -> Self {
        Self {
            call_repository,
            voice_service,
            friendship_repository,
        }
    }

    pub async fn execute(
        &self,
        blocker: UserId,
        blocked: UserId,
        request_id: Option<String>,
    ) -> Result<(), EnforceBlockError> {
        if !self.friendship_repository.is_blocked(&blocker, &blocked).await? {
            return Ok(());
        }
        for call_id in self
            .call_repository
            .shared_active_calls(&blocker, &blocked)
            .await?
        {
            self.evict(call_id, &blocker, &blocked, request_id.clone())
                .await?;
        }
        Ok(())
    }

    async fn evict(
        &self,
        call_id: Uuid,
        blocker: &UserId,
        blocked: &UserId,
        request_id: Option<String>,
    ) -> Result<(), EnforceBlockError> {
        let others = self
            .call_repository
            .list_active(call_id)
            .await?
            .into_iter()
            .filter(|p| p.user_id != *blocker && p.user_id != *blocked)
            .count();
        let mut leaving = vec![*blocker];
        if others == 0 {
            leaving.push(*blocked);
        }
        let room = ChannelId::new(call_id);
        for user in &leaving {
            self.voice_service.remove_participant(&room, user).await?;
        }
        self.call_repository
            .leave_many(call_id, &leaving, Utc::now(), request_id)
            .await?;
        Ok(())
    }
}
