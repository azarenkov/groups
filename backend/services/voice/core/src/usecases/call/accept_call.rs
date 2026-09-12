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

struct BlockPlan {
    user_out: bool,
    blockers: Vec<UserId>,
}

struct BlockOutcome {
    user_out: bool,
    evicted: bool,
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

        let room = ChannelId::new(call_id);
        if self.plan_blocks(&user_id, &checked).await?.user_out {
            self.call_repository
                .respond(call_id, &user_id, false, Utc::now(), request_id)
                .await?;
            return Err(AcceptCallError::CallNotAllowed);
        }

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
        let roster: Vec<UserId> = participants.iter().map(|p| p.user_id).collect();
        let outcome = match self
            .resolve_blocks(call_id, &room, &user_id, &roster, request_id.clone())
            .await
        {
            Ok(outcome) => outcome,
            Err(e) => {
                if let Err(rollback) = self.roll_back(call_id, &room, &user_id, request_id).await {
                    tracing::error!(error = %rollback, %call_id, "Failed to roll back a join after a block lookup error");
                }
                return Err(e);
            }
        };
        if outcome.user_out {
            self.roll_back(call_id, &room, &user_id, request_id).await?;
            return Err(AcceptCallError::CallNotAllowed);
        }
        if outcome.evicted {
            participants = self
                .call_repository
                .list_active(call_id)
                .await?
                .into_iter()
                .filter(|p| p.user_id != user_id)
                .collect();
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

    async fn plan_blocks(
        &self,
        user_id: &UserId,
        roster: &[UserId],
    ) -> Result<BlockPlan, FriendshipLookupError> {
        let mut blockers: Vec<UserId> = Vec::new();
        for other in roster {
            let relation = self.friendship_repository.block_relation(user_id, other).await?;
            if relation.blocking {
                return Ok(BlockPlan { user_out: true, blockers });
            }
            if relation.blocked_by {
                blockers.push(*other);
            }
        }
        let user_out = !blockers.is_empty() && blockers.len() == roster.len();
        Ok(BlockPlan { user_out, blockers })
    }

    async fn resolve_blocks(
        &self,
        call_id: Uuid,
        _room: &ChannelId,
        user_id: &UserId,
        roster: &[UserId],
        request_id: Option<String>,
    ) -> Result<BlockOutcome, AcceptCallError> {
        let plan = self.plan_blocks(user_id, roster).await?;
        if plan.user_out {
            return Ok(BlockOutcome { user_out: true, evicted: false });
        }
        if plan.blockers.is_empty() {
            return Ok(BlockOutcome { user_out: false, evicted: false });
        }
        super::eviction::evict(
            self.call_repository.as_ref(),
            self.voice_service.as_ref(),
            call_id,
            &plan.blockers,
            request_id,
        )
        .await?;
        Ok(BlockOutcome { user_out: false, evicted: true })
    }

    async fn roll_back(
        &self,
        call_id: Uuid,
        _room: &ChannelId,
        user_id: &UserId,
        request_id: Option<String>,
    ) -> Result<(), RepositoryError> {
        super::eviction::evict(
            self.call_repository.as_ref(),
            self.voice_service.as_ref(),
            call_id,
            &[*user_id],
            request_id,
        )
        .await
    }
}
