use std::sync::Arc;

use chrono::Utc;
use voice_domain::{call::repository::CallParticipantView, call::repository::CallRepository};
use domain::{channel::id::ChannelId, channel::token::ChannelToken, shared::repository::error::RepositoryError, user::id::UserId};
use uuid::Uuid;

use crate::ports::voice_service::{VoiceService, VoiceServiceError};

#[derive(Debug, thiserror::Error)]
pub enum AcceptCallError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Voice service error: {0}")]
    Voice(#[from] VoiceServiceError),
    #[error("You are not invited to this call")]
    NotParticipant,
    #[error("User not found")]
    UserNotFound,
}

pub struct AcceptCallResult {
    pub token: ChannelToken,
    pub url: String,
    pub participants: Vec<CallParticipantView>,
}

pub struct AcceptCall<C, V, U>
where
    C: CallRepository,
    V: VoiceService,
    U: domain::user_profile::UserProfileRepository,
{
    call_repository: Arc<C>,
    voice_service: Arc<V>,
    user_repository: Arc<U>,
}

impl<C, V, U> AcceptCall<C, V, U>
where
    C: CallRepository,
    V: VoiceService,
    U: domain::user_profile::UserProfileRepository,
{
    pub fn new(
        call_repository: Arc<C>,
        voice_service: Arc<V>,
        user_repository: Arc<U>,
    ) -> Self {
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
        request_id: Option<String>,
    ) -> Result<AcceptCallResult, AcceptCallError> {
        if !self
            .call_repository
            .is_active_participant(call_id, &user_id)
            .await?
        {
            return Err(AcceptCallError::NotParticipant);
        }

        let user_name = domain::user::name::UserName::new(
            self.user_repository
                .get_name(&user_id)
                .await?
                .ok_or(AcceptCallError::UserNotFound)?,
        );

        let room = ChannelId::new(call_id);
        let token = self
            .voice_service
            .create_token(&user_id, &user_name, &room)
            .await?;

        let mut participants: Vec<CallParticipantView> = self
            .call_repository
            .list_active(call_id)
            .await?
            .into_iter()
            .filter(|p| p.user_id != user_id)
            .collect();

        let accepted = self
            .call_repository
            .respond(call_id, &user_id, true, Utc::now(), request_id)
            .await?;

        if !accepted
            && !self
                .call_repository
                .is_joined_participant(call_id, &user_id)
                .await?
        {
            return Err(AcceptCallError::NotParticipant);
        }

        participants.push(CallParticipantView {
            user_id: UserId::new(user_id.uuid()),
            user_name: user_name.as_str().to_string(),
        });

        Ok(AcceptCallResult {
            token,
            url: self.voice_service.client_url(),
            participants,
        })
    }
}
