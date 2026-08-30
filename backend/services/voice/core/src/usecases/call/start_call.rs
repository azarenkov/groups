use std::sync::Arc;

use chrono::Utc;
use voice_domain::{call::repository::CallParticipantView, call::repository::CallRepository};
use domain::{channel::id::ChannelId, channel::token::ChannelToken, server::id::ServerId, user::id::UserId};
use ports::repository::RepositoryError;
use uuid::Uuid;

use crate::{
    dto::requests::call::start_call::StartCallRequest,
    ports::{
        friendship_lookup::{FriendshipLookup, FriendshipLookupError},
        voice_service::{VoiceService, VoiceServiceError},
    },
};

#[derive(Debug, thiserror::Error)]
pub enum StartCallError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Voice service error: {0}")]
    Voice(#[from] VoiceServiceError),
    #[error("No recipients specified")]
    NoRecipients,
    #[error("You can only call your friends")]
    NotFriends,
    #[error("User not found")]
    UserNotFound,
    #[error("Account is inactive")]
    AccountInactive,
    #[error("Friendship lookup unavailable: {0}")]
    FriendshipLookup(#[from] FriendshipLookupError),
}

pub struct StartCallResult {
    pub call_id: Uuid,
    pub token: ChannelToken,
    pub url: String,
    pub participants: Vec<CallParticipantView>,
}

pub struct StartCall<C, V, F, U>
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

impl<C, V, F, U> StartCall<C, V, F, U>
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
        caller_id: UserId,
        request: StartCallRequest,
    ) -> Result<StartCallResult, StartCallError> {
        if request.invitee_ids.is_empty() {
            return Err(StartCallError::NoRecipients);
        }

        let mut invitees: Vec<UserId> = Vec::new();
        for id in &request.invitee_ids {
            let invitee = UserId::new(*id);
            if invitee == caller_id {
                continue;
            }
            if !self
                .friendship_repository
                .are_friends(&caller_id, &invitee)
                .await?
            {
                return Err(StartCallError::NotFriends);
            }
            invitees.push(invitee);
        }
        if invitees.is_empty() {
            return Err(StartCallError::NoRecipients);
        }

        let caller_name = domain::user::name::UserName::new(
            self.user_repository
                .get_name(&caller_id)
                .await?
                .ok_or(StartCallError::UserNotFound)?,
        );

        if !self.user_repository.is_active(&caller_id).await? {
            return Err(StartCallError::AccountInactive);
        }
        let call_id = Uuid::new_v4();
        let room = ChannelId::new(call_id);
        self.voice_service
            .create_voice_channel(&room, &ServerId::new(call_id))
            .await?;

        let token = self
            .voice_service
            .create_token(&caller_id, &caller_name, &room)
            .await?;

        let participants = vec![CallParticipantView {
            user_id: UserId::new(caller_id.uuid()),
            user_name: caller_name.as_str().to_string(),
        }];

        self.call_repository
            .start_call(
                call_id,
                &caller_id,
                &invitees,
                Utc::now(),
                request.ctx.request_id.clone(),
            )
            .await?;
        if !self.user_repository.is_active(&caller_id).await? {
            self.call_repository
                .leave(call_id, &caller_id, Utc::now(), request.ctx.request_id)
                .await?;
            let _ = self.voice_service.remove_participant(&room, &caller_id).await;
            let _ = self.voice_service.delete_voice_channel(&room).await;
            return Err(StartCallError::AccountInactive);
        }

        Ok(StartCallResult {
            call_id,
            token,
            url: self.voice_service.client_url(),
            participants,
        })
    }
}
