use std::sync::Arc;

use chrono::Utc;
use domain::{channel::id::ChannelId, user::id::UserId};
use ports::repository::RepositoryError;
use uuid::Uuid;
use voice_domain::call::repository::CallRepository;

use crate::ports::voice_service::{VoiceService, VoiceServiceError};

#[derive(Debug, thiserror::Error)]
pub enum EnforceBlockError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Voice service error: {0}")]
    Voice(#[from] VoiceServiceError),
}

pub struct EnforceBlock<C, V>
where
    C: CallRepository,
    V: VoiceService,
{
    call_repository: Arc<C>,
    voice_service: Arc<V>,
}

impl<C, V> EnforceBlock<C, V>
where
    C: CallRepository,
    V: VoiceService,
{
    pub fn new(call_repository: Arc<C>, voice_service: Arc<V>) -> Self {
        Self {
            call_repository,
            voice_service,
        }
    }

    pub async fn execute(
        &self,
        blocker: UserId,
        blocked: UserId,
        request_id: Option<String>,
    ) -> Result<(), EnforceBlockError> {
        let Some(call_id) = self.shared_call(&blocker, &blocked).await? else {
            return Ok(());
        };
        let others = self
            .call_repository
            .list_active(call_id)
            .await?
            .into_iter()
            .filter(|p| p.user_id != blocker && p.user_id != blocked)
            .count();
        let mut leaving = vec![blocker];
        if others == 0 {
            leaving.push(blocked);
        }
        let room = ChannelId::new(call_id);
        for user in leaving {
            let _ = self.voice_service.remove_participant(&room, &user).await;
            self.call_repository
                .leave(call_id, &user, Utc::now(), request_id.clone())
                .await?;
        }
        Ok(())
    }

    async fn shared_call(&self, a: &UserId, b: &UserId) -> Result<Option<Uuid>, RepositoryError> {
        for (joined, other) in [(a, b), (b, a)] {
            if let Some(call_id) = self.call_repository.active_call_for(joined).await?
                && self.call_repository.is_active_participant(call_id, other).await?
            {
                return Ok(Some(call_id));
            }
        }
        Ok(None)
    }
}
