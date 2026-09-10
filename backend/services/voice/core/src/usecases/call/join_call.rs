use std::sync::Arc;

use chrono::Utc;
use voice_domain::{call::repository::CallParticipantView, call::repository::CallRepository};
use domain::{channel::id::ChannelId, channel::token::ChannelToken, user::id::UserId};
use ports::repository::RepositoryError;
use uuid::Uuid;

use crate::ports::{
    friendship_lookup::{FriendshipLookup, FriendshipLookupError},
    voice_service::{VoiceService, VoiceServiceError},
};

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
    #[error("Account is inactive")]
    AccountInactive,
    #[error("This user does not accept this call")]
    CallNotAllowed,
    #[error("Friendship lookup unavailable: {0}")]
    FriendshipLookup(#[from] FriendshipLookupError),
}

pub struct JoinCallResult {
    pub token: ChannelToken,
    pub url: String,
    pub participants: Vec<CallParticipantView>,
    pub video: bool,
}

pub struct JoinCall<C, V, F, U>
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

impl<C, V, F, U> JoinCall<C, V, F, U>
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
        call_id: Uuid,
    ) -> Result<JoinCallResult, JoinCallError> {
        let user_name = domain::user::name::UserName::new(
            self.user_repository
                .get_name(&user_id)
                .await?
                .ok_or(JoinCallError::UserNotFound)?,
        );

        if !self.user_repository.is_active(&user_id).await? {
            return Err(JoinCallError::AccountInactive);
        }

        let checked: Vec<UserId> = self
            .call_repository
            .list_active(call_id)
            .await?
            .into_iter()
            .map(|p| p.user_id)
            .filter(|id| *id != user_id)
            .collect();

        for other in &checked {
            if self.friendship_repository.is_blocked(&user_id, other).await? {
                return Err(JoinCallError::CallNotAllowed);
            }
        }

        let room = ChannelId::new(call_id);
        let token = self
            .voice_service
            .create_token(&user_id, &user_name, &room)
            .await?;

        let joined = self
            .call_repository
            .join(call_id, &user_id, Utc::now())
            .await?;
        if !joined {
            return Err(JoinCallError::NotParticipant);
        }
        if !self.user_repository.is_active(&user_id).await? {
            self.call_repository
                .leave(call_id, &user_id, Utc::now(), None)
                .await?;
            let _ = self.voice_service.remove_participant(&room, &user_id).await;
            return Err(JoinCallError::AccountInactive);
        }

        let mut participants: Vec<CallParticipantView> = self
            .call_repository
            .list_active(call_id)
            .await?
            .into_iter()
            .filter(|p| p.user_id != user_id)
            .collect();
        for participant in participants.iter().filter(|p| !checked.contains(&p.user_id)) {
            if self
                .friendship_repository
                .is_blocked(&user_id, &participant.user_id)
                .await?
            {
                self.voice_service.remove_participant(&room, &user_id).await?;
                self.call_repository
                    .leave(call_id, &user_id, Utc::now(), None)
                    .await?;
                return Err(JoinCallError::CallNotAllowed);
            }
        }

        participants.push(CallParticipantView {
            user_id: UserId::new(user_id.uuid()),
            user_name: user_name.as_str().to_string(),
        });        let video = self.call_repository.call_video(call_id).await?;


        Ok(JoinCallResult {
            token,
            url: self.voice_service.client_url(),
            participants,
        video,
        })
    }
}
