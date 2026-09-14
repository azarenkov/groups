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

pub const MAX_CALL_INVITEES: usize = 24;

#[derive(Debug, thiserror::Error)]
pub enum StartCallError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Voice service error: {0}")]
    Voice(#[from] VoiceServiceError),
    #[error("No recipients specified")]
    NoRecipients,
    #[error("Too many recipients")]
    TooManyRecipients,
    #[error("This user does not accept this call")]
    CallNotAllowed,
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
    pub video: bool,
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

        let mut requested: Vec<UserId> = Vec::new();
        for id in &request.invitee_ids {
            let invitee = UserId::new(*id);
            if invitee == caller_id || requested.contains(&invitee) {
                continue;
            }
            if requested.len() == MAX_CALL_INVITEES {
                return Err(StartCallError::TooManyRecipients);
            }
            requested.push(invitee);
        }
        let mut invitees: Vec<UserId> = Vec::new();
        for invitee in requested {
            if !self
                .friendship_repository
                .can_call(&caller_id, &invitee)
                .await?
            {
                return Err(StartCallError::CallNotAllowed);
            }
            if !self.user_repository.is_active(&invitee).await? {
                return Err(StartCallError::CallNotAllowed);
            }
            invitees.push(invitee);
        }
        if invitees.is_empty() {
            return Err(StartCallError::NoRecipients);
        }
        for (index, a) in invitees.iter().enumerate() {
            for b in &invitees[index + 1..] {
                if self.friendship_repository.is_blocked(a, b).await? {
                    return Err(StartCallError::CallNotAllowed);
                }
            }
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

        let video = request.video;
        self.call_repository
            .start_call(
                call_id,
                &caller_id,
                &invitees,
                video,
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

        let remaining = match self
            .evict_blocked_invitees(call_id, &caller_id, &invitees, request.ctx.request_id.clone())
            .await
        {
            Ok(Some(remaining)) => remaining,
            Ok(None) => 0,
            Err(e) => {
                let mut everyone = invitees.clone();
                everyone.push(caller_id);
                if let Err(cleanup) = self
                    .call_repository
                    .leave_many(call_id, &everyone, Utc::now(), request.ctx.request_id)
                    .await
                {
                    tracing::error!(error = %cleanup, %call_id, "Failed to abort a call after a block lookup error");
                }
                let _ = self.voice_service.delete_voice_channel(&room).await;
                return Err(e);
            }
        };
        if remaining == 0 {
            let mut everyone = invitees.clone();
            everyone.push(caller_id);
            self.call_repository
                .leave_many(call_id, &everyone, Utc::now(), request.ctx.request_id)
                .await?;
            if let Err(e) = self.voice_service.remove_participant(&room, &caller_id).await {
                tracing::warn!(error = %e, %call_id, "Failed to remove the caller from an aborted call room");
            }
            let _ = self.voice_service.delete_voice_channel(&room).await;
            return Err(StartCallError::CallNotAllowed);
        }

        Ok(StartCallResult {
            call_id,
            token,
            url: self.voice_service.client_url(),
            participants,
            video,
        })
    }

    async fn evict_blocked_invitees(
        &self,
        call_id: Uuid,
        caller_id: &UserId,
        invitees: &[UserId],
        request_id: Option<String>,
    ) -> Result<Option<usize>, StartCallError> {
        let mut kept: Vec<UserId> = Vec::new();
        for invitee in invitees {
            let relation = self.friendship_repository.block_relation(caller_id, invitee).await?;
            if relation.blocking {
                return Ok(None);
            }
            if relation.blocked_by {
                super::eviction::evict(
                    self.call_repository.as_ref(),
                    self.voice_service.as_ref(),
                    call_id,
                    &[*invitee],
                    request_id.clone(),
                )
                .await?;
            } else {
                kept.push(*invitee);
            }
        }
        let mut index = 0;
        while index < kept.len() {
            let mut evicted_current = false;
            let mut other = index + 1;
            while other < kept.len() {
                let relation = self
                    .friendship_repository
                    .block_relation(&kept[index], &kept[other])
                    .await?;
                if !relation.any() {
                    other += 1;
                    continue;
                }
                let blocker_position = if relation.blocking { index } else { other };
                let blocker = kept.remove(blocker_position);
                super::eviction::evict(
                    self.call_repository.as_ref(),
                    self.voice_service.as_ref(),
                    call_id,
                    &[blocker],
                    request_id.clone(),
                )
                .await?;
                if blocker_position == index {
                    evicted_current = true;
                    break;
                }
            }
            if !evicted_current {
                index += 1;
            }
        }
        Ok(Some(kept.len()))
    }
}
