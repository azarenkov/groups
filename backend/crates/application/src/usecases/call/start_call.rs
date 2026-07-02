use std::sync::Arc;

use chrono::Utc;
use domain::{
    call::repository::{CallParticipantView, CallRepository},
    channel::{id::ChannelId, token::ChannelToken},
    friendship::{repository::FriendshipRepository, status::FriendshipStatus},
    server::id::ServerId,
    shared::repository::error::RepositoryError,
    user::{id::UserId, repository::UserRepository},
};
use uuid::Uuid;

use crate::{
    dto::requests::call::start_call::StartCallRequest,
    ports::voice_service::{VoiceService, VoiceServiceError},
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
    F: FriendshipRepository,
    U: UserRepository,
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
    F: FriendshipRepository,
    U: UserRepository,
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
            let pair = self
                .friendship_repository
                .find_pair(&caller_id, &invitee)
                .await?;
            if !matches!(pair, Some(ref f) if f.status == FriendshipStatus::Accepted) {
                return Err(StartCallError::NotFriends);
            }
            invitees.push(invitee);
        }
        if invitees.is_empty() {
            return Err(StartCallError::NoRecipients);
        }

        let caller = self
            .user_repository
            .find_by_id(&caller_id)
            .await?
            .ok_or(StartCallError::UserNotFound)?;

        let call_id = Uuid::new_v4();
        let room = ChannelId::new(call_id);
        self.voice_service
            .create_voice_channel(&room, &ServerId::new(call_id))
            .await?;

        let token = self
            .voice_service
            .create_token(&caller_id, caller.name(), &room)
            .await?;

        let participants = vec![CallParticipantView {
            user_id: UserId::new(caller_id.uuid()),
            user_name: caller.name().as_str().to_string(),
        }];

        self.call_repository
            .start_call(
                call_id,
                &caller_id,
                &invitees,
                Utc::now(),
                request.ctx.request_id,
            )
            .await?;

        Ok(StartCallResult {
            call_id,
            token,
            url: self.voice_service.client_url(),
            participants,
        })
    }
}
