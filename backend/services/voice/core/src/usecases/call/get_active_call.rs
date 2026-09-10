use std::sync::Arc;

use chrono::{DateTime, Utc};
use voice_domain::{call::repository::CallParticipantView, call::repository::CallRepository};
use domain::{channel::id::ChannelId, channel::token::ChannelToken, user::id::UserId};
use ports::repository::RepositoryError;
use uuid::Uuid;

use crate::ports::{
    friendship_lookup::{FriendshipLookup, FriendshipLookupError},
    voice_service::{VoiceService, VoiceServiceError},
};

#[derive(Debug, thiserror::Error)]
pub enum GetActiveCallError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Voice service error: {0}")]
    Voice(#[from] VoiceServiceError),
    #[error("User not found")]
    UserNotFound,
    #[error("Friendship lookup unavailable: {0}")]
    FriendshipLookup(#[from] FriendshipLookupError),
}

pub struct ActiveCallResult {
    pub call_id: Uuid,
    pub token: ChannelToken,
    pub url: String,
    pub participants: Vec<CallParticipantView>,
    pub video: bool,
    pub started_at: Option<DateTime<Utc>>,
}

pub struct GetActiveCall<C, V, F, U>
where
    C: CallRepository,
    V: VoiceService,
    F: FriendshipLookup,
    U: ports::user_profile::UserProfileRepository,
{
    call_repository: Arc<C>,
    voice_service: Arc<V>,
    friendship_repository: Arc<F>,
    user_repository: Arc<U>,
}

impl<C, V, F, U> GetActiveCall<C, V, F, U>
where
    C: CallRepository,
    V: VoiceService,
    F: FriendshipLookup,
    U: ports::user_profile::UserProfileRepository,
{
    pub fn new(
        call_repository: Arc<C>,
        voice_service: Arc<V>,
        friendship_repository: Arc<F>,
        user_repository: Arc<U>,
    ) -> Self {
        Self {
            call_repository,
            voice_service,
            friendship_repository,
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

        let user_name = domain::user::name::UserName::new(
            self.user_repository
                .get_name(&user_id)
                .await?
                .ok_or(GetActiveCallError::UserNotFound)?,
        );

        let room = ChannelId::new(call_id);
        let participants = self.call_repository.list_active(call_id).await?;
        for participant in participants.iter().filter(|p| p.user_id != user_id) {
            if self
                .friendship_repository
                .is_blocked(&user_id, &participant.user_id)
                .await?
            {
                let _ = self.voice_service.remove_participant(&room, &user_id).await;
                self.call_repository
                    .leave(call_id, &user_id, Utc::now(), None)
                    .await?;
                return Ok(None);
            }
        }
        let token = self
            .voice_service
            .create_token(&user_id, &user_name, &room)
            .await?;

        let video = self.call_repository.call_video(call_id).await?;
        let started_at = self.call_repository.joined_at(call_id, &user_id).await?;

        Ok(Some(ActiveCallResult {
            call_id,
            token,
            url: self.voice_service.client_url(),
            participants,
            video,
            started_at,
        }))
    }
}
