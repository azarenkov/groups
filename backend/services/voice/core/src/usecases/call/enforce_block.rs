use std::sync::Arc;

use domain::user::id::UserId;
use ports::repository::RepositoryError;
use uuid::Uuid;
use voice_domain::call::repository::CallRepository;

use super::eviction::evict;
use crate::ports::{
    friendship_lookup::FriendshipLookup,
    voice_service::{VoiceService, VoiceServiceError},
};

#[derive(Debug, thiserror::Error)]
pub enum EnforceBlockError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Voice service error: {0}")]
    Voice(#[from] VoiceServiceError),
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
        let blocking = match self.friendship_repository.block_relation(&blocker, &blocked).await {
            Ok(relation) => relation.blocking,
            Err(e) => {
                tracing::warn!(error = %e, blocker = %blocker.uuid(), blocked = %blocked.uuid(), "Block lookup unavailable; enforcing the block event as delivered");
                true
            }
        };
        if !blocking {
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
        evict(
            self.call_repository.as_ref(),
            self.voice_service.as_ref(),
            call_id,
            &leaving,
            request_id,
        )
        .await?;
        Ok(())
    }
}
