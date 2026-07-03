use std::sync::Arc;

use chrono::Utc;
use domain::{
    call::repository::{CallParticipantView, CallRepository},
    channel::{id::ChannelId, token::ChannelToken},
    shared::repository::error::RepositoryError,
    user::{id::UserId, repository::UserRepository},
};
use uuid::Uuid;

use crate::ports::voice_service::{VoiceService, VoiceServiceError};

#[derive(Debug, thiserror::Error)]
pub enum JoinCallError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Voice service error: {0}")]
    Voice(#[from] VoiceServiceError),
    #[error("You are not part of this call")]
    NotParticipant,
    #[error("User not found")]
    UserNotFound,
}

pub struct JoinCallResult {
    pub token: ChannelToken,
    pub url: String,
    pub participants: Vec<CallParticipantView>,
}

pub struct JoinCall<C, V, U>
where
    C: CallRepository,
    V: VoiceService,
    U: UserRepository,
{
    call_repository: Arc<C>,
    voice_service: Arc<V>,
    user_repository: Arc<U>,
}

impl<C, V, U> JoinCall<C, V, U>
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
        call_id: Uuid,
    ) -> Result<JoinCallResult, JoinCallError> {
        let user = self
            .user_repository
            .find_by_id(&user_id)
            .await?
            .ok_or(JoinCallError::UserNotFound)?;

        let room = ChannelId::new(call_id);
        let token = self
            .voice_service
            .create_token(&user_id, user.name(), &room)
            .await?;

        let mut participants: Vec<CallParticipantView> = self
            .call_repository
            .list_active(call_id)
            .await?
            .into_iter()
            .filter(|p| p.user_id != user_id)
            .collect();

        let joined = self
            .call_repository
            .join(call_id, &user_id, Utc::now())
            .await?;
        if !joined {
            return Err(JoinCallError::NotParticipant);
        }

        participants.push(CallParticipantView {
            user_id: UserId::new(user_id.uuid()),
            user_name: user.name().as_str().to_string(),
        });

        Ok(JoinCallResult {
            token,
            url: self.voice_service.client_url(),
            participants,
        })
    }
}
