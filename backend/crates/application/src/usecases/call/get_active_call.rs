use std::sync::Arc;

use domain::{
    call::repository::{CallParticipantView, CallRepository},
    channel::{id::ChannelId, token::ChannelToken},
    shared::repository::error::RepositoryError,
    user::{id::UserId, repository::UserRepository},
};
use uuid::Uuid;

use crate::ports::voice_service::{VoiceService, VoiceServiceError};

#[derive(Debug, thiserror::Error)]
pub enum GetActiveCallError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Voice service error: {0}")]
    Voice(#[from] VoiceServiceError),
    #[error("User not found")]
    UserNotFound,
}

pub struct ActiveCallResult {
    pub call_id: Uuid,
    pub token: ChannelToken,
    pub url: String,
    pub participants: Vec<CallParticipantView>,
}

pub struct GetActiveCall<C, V, U>
where
    C: CallRepository,
    V: VoiceService,
    U: UserRepository,
{
    call_repository: Arc<C>,
    voice_service: Arc<V>,
    user_repository: Arc<U>,
}

impl<C, V, U> GetActiveCall<C, V, U>
where
    C: CallRepository,
    V: VoiceService,
    U: UserRepository,
{
    pub fn new(call_repository: Arc<C>, voice_service: Arc<V>, user_repository: Arc<U>) -> Self {
        Self {
            call_repository,
            voice_service,
            user_repository,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
    ) -> Result<Option<ActiveCallResult>, GetActiveCallError> {
        let call_id = match self.call_repository.active_call_for(&user_id).await? {
            Some(id) => id,
            None => return Ok(None),
        };

        let user = self
            .user_repository
            .find_by_id(&user_id)
            .await?
            .ok_or(GetActiveCallError::UserNotFound)?;

        let room = ChannelId::new(call_id);
        let token = self
            .voice_service
            .create_token(&user_id, user.name(), &room)
            .await?;

        let participants = self.call_repository.list_active(call_id).await?;

        Ok(Some(ActiveCallResult {
            call_id,
            token,
            url: self.voice_service.client_url(),
            participants,
        }))
    }
}
