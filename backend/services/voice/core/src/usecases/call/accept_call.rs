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
pub enum AcceptCallError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Voice service error: {0}")]
    Voice(#[from] VoiceServiceError),
    #[error("You are not invited to this call")]
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

pub struct AcceptCallResult {
    pub token: ChannelToken,
    pub url: String,
    pub participants: Vec<CallParticipantView>,
    pub video: bool,
}

pub struct AcceptCall<C, V, F, U>
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

impl<C, V, F, U> AcceptCall<C, V, F, U>
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

        if !self.user_repository.is_active(&user_id).await? {
            return Err(AcceptCallError::AccountInactive);
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
                self.call_repository
                    .respond(call_id, &user_id, false, Utc::now(), request_id)
                    .await?;
                return Err(AcceptCallError::CallNotAllowed);
            }
        }

        let room = ChannelId::new(call_id);
        let token = self
            .voice_service
            .create_token(&user_id, &user_name, &room)
            .await?;

        let accepted = self
            .call_repository
            .respond(call_id, &user_id, true, Utc::now(), request_id.clone())
            .await?;
        if !self.user_repository.is_active(&user_id).await? {
            self.call_repository
                .leave(call_id, &user_id, Utc::now(), request_id)
                .await?;
            let _ = self.voice_service.remove_participant(&room, &user_id).await;
            return Err(AcceptCallError::AccountInactive);
        }

        if !accepted
            && !self
                .call_repository
                .is_joined_participant(call_id, &user_id)
                .await?
        {
            return Err(AcceptCallError::NotParticipant);
        }

        let mut participants: Vec<CallParticipantView> = self
            .call_repository
            .list_active(call_id)
            .await?
            .into_iter()
            .filter(|p| p.user_id != user_id)
            .collect();
        let newcomers: Vec<UserId> = participants
            .iter()
            .filter(|p| !checked.contains(&p.user_id))
            .map(|p| p.user_id)
            .collect();
        let blocked = match self.blocked_with_any(&user_id, &newcomers).await {
            Ok(blocked) => blocked,
            Err(e) => {
                self.roll_back(call_id, &room, &user_id, request_id).await;
                return Err(e.into());
            }
        };
        if blocked {
            self.voice_service.remove_participant(&room, &user_id).await?;
            self.call_repository
                .leave(call_id, &user_id, Utc::now(), request_id)
                .await?;
            return Err(AcceptCallError::CallNotAllowed);
        }

        participants.push(CallParticipantView {
            user_id: UserId::new(user_id.uuid()),
            user_name: user_name.as_str().to_string(),
        });        let video = self.call_repository.call_video(call_id).await?;


        Ok(AcceptCallResult {
            token,
            url: self.voice_service.client_url(),
            participants,
        video,
        })
    }

    async fn blocked_with_any(
        &self,
        user_id: &UserId,
        others: &[UserId],
    ) -> Result<bool, FriendshipLookupError> {
        for other in others {
            if self.friendship_repository.is_blocked(user_id, other).await? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn roll_back(
        &self,
        call_id: Uuid,
        room: &ChannelId,
        user_id: &UserId,
        request_id: Option<String>,
    ) {
        if let Err(e) = self.voice_service.remove_participant(room, user_id).await {
            tracing::warn!(error = %e, %call_id, "Failed to remove a participant while rolling back a join");
        }
        if let Err(e) = self
            .call_repository
            .leave(call_id, user_id, Utc::now(), request_id)
            .await
        {
            tracing::error!(error = %e, %call_id, "Failed to roll back a join after a block lookup error");
        }
    }
}
