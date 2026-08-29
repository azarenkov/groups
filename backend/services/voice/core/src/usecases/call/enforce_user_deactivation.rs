use std::sync::Arc;

use chrono::Utc;
use domain::{channel::id::ChannelId, user::id::UserId};
use ports::repository::RepositoryError;
use voice_domain::call::repository::CallRepository;

use crate::ports::voice_service::{VoiceService, VoiceServiceError};

#[derive(Debug, thiserror::Error)]
pub enum EnforceUserDeactivationError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Voice service error: {0}")]
    Voice(#[from] VoiceServiceError),
}

pub struct EnforceUserDeactivation<C, V>
where
    C: CallRepository,
    V: VoiceService,
{
    call_repository: Arc<C>,
    voice_service: Arc<V>,
}

impl<C, V> EnforceUserDeactivation<C, V>
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

    pub async fn execute(&self, user_id: UserId, request_id: Option<String>) -> Result<(), EnforceUserDeactivationError> {
        let Some(call_id) = self.call_repository.active_call_for(&user_id).await? else {
            return Ok(());
        };
        self.voice_service
            .remove_participant(&ChannelId::new(call_id), &user_id)
            .await?;
        self.call_repository
            .leave(call_id, &user_id, Utc::now(), request_id)
            .await?;
        Ok(())
    }
}
