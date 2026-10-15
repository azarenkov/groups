use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;
use voice_domain::call::repository::{AddParticipantsOutcome, CallRepository};
use domain::user::id::UserId;
use ports::repository::RepositoryError;

use crate::{
    dto::requests::call::add_participants::AddCallParticipantsRequest,
    ports::{
        friendship_lookup::{FriendshipLookup, FriendshipLookupError},
        voice_service::VoiceService,
    },
};

pub const MAX_CALL_INVITEES: usize = 24;

#[derive(Debug, thiserror::Error)]
pub enum AddCallParticipantsError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("You are not part of this call")]
    NotParticipant,
    #[error("This user cannot be added to this call")]
    CallNotAllowed,
    #[error("No recipients specified")]
    NoRecipients,
    #[error("Too many recipients")]
    TooManyRecipients,
    #[error("Friendship lookup unavailable: {0}")]
    FriendshipLookup(#[from] FriendshipLookupError),
}

pub struct AddCallParticipants<C, V, F, U>
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

impl<C, V, F, U> AddCallParticipants<C, V, F, U>
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
        request: AddCallParticipantsRequest,
    ) -> Result<(), AddCallParticipantsError> {
        if !self
            .call_repository
            .is_joined_participant(request.call_id, &caller_id)
            .await?
        {
            return Err(AddCallParticipantsError::NotParticipant);
        }

        let roster = self.roster(request.call_id).await?;
        let mut requested: Vec<UserId> = Vec::new();
        for id in &request.invitee_ids {
            let invitee = UserId::new(*id);
            if invitee == caller_id || requested.contains(&invitee) {
                continue;
            }
            if requested.len() == MAX_CALL_INVITEES {
                return Err(AddCallParticipantsError::TooManyRecipients);
            }
            requested.push(invitee);
        }
        let existing = roster.iter().filter(|id| **id != caller_id).count();
        let fresh = requested.iter().filter(|id| !roster.contains(id)).count();
        if existing + fresh > MAX_CALL_INVITEES {
            return Err(AddCallParticipantsError::TooManyRecipients);
        }
        let mut invitees: Vec<UserId> = Vec::new();
        for invitee in requested {
            if !self
                .friendship_repository
                .can_call(&caller_id, &invitee)
                .await?
            {
                return Err(AddCallParticipantsError::CallNotAllowed);
            }
            if self.blocked_with_roster(&invitee, &roster).await? {
                return Err(AddCallParticipantsError::CallNotAllowed);
            }
            if !self.user_repository.is_active(&invitee).await? {
                return Err(AddCallParticipantsError::CallNotAllowed);
            }
            invitees.push(invitee);
        }
        if invitees.is_empty() {
            return Err(AddCallParticipantsError::NoRecipients);
        }
        if self.blocked_among(&invitees).await? {
            return Err(AddCallParticipantsError::CallNotAllowed);
        }

        let added = match self
            .call_repository
            .add_participants(
                request.call_id,
                &caller_id,
                &invitees,
                MAX_CALL_INVITEES,
                Utc::now(),
                request.ctx.request_id.clone(),
            )
            .await?
        {
            AddParticipantsOutcome::Added(added) => added,
            AddParticipantsOutcome::CapExceeded => {
                return Err(AddCallParticipantsError::TooManyRecipients);
            }
            AddParticipantsOutcome::InviterLeft => {
                return Err(AddCallParticipantsError::NotParticipant);
            }
        };

        let roster = match self.roster(request.call_id).await {
            Ok(roster) => roster,
            Err(e) => {
                self.withdraw(request.call_id, &added, request.ctx.request_id).await;
                return Err(e.into());
            }
        };
        let mut withdrawn: Vec<UserId> = match self.blockers_among(&added).await {
            Ok(list) => list,
            Err(e) => {
                self.withdraw(request.call_id, &added, request.ctx.request_id).await;
                return Err(e.into());
            }
        };
        let candidates: Vec<UserId> = added.iter().copied().filter(|i| !withdrawn.contains(i)).collect();
        let mut roster = roster;
        for invitee in &candidates {
            match self
                .reconcile_with_roster(request.call_id, invitee, &mut roster, request.ctx.request_id.clone())
                .await
            {
                Ok(true) => withdrawn.push(*invitee),
                Ok(false) => {}
                Err(e) => {
                    let pending: Vec<UserId> = added.iter().copied().filter(|i| !withdrawn.contains(i)).collect();
                    self.withdraw(request.call_id, &pending, request.ctx.request_id).await;
                    return Err(e);
                }
            }
        }
        if !withdrawn.is_empty() {
            super::eviction::evict(
                self.call_repository.as_ref(),
                self.voice_service.as_ref(),
                request.call_id,
                &withdrawn,
                request.ctx.request_id,
            )
            .await?;
        }
        if !added.is_empty() && withdrawn.len() == added.len() {
            return Err(AddCallParticipantsError::CallNotAllowed);
        }
        Ok(())
    }

    async fn blockers_among(&self, users: &[UserId]) -> Result<Vec<UserId>, FriendshipLookupError> {
        let mut blockers: Vec<UserId> = Vec::new();
        for (index, a) in users.iter().enumerate() {
            for b in &users[index + 1..] {
                let relation = self.friendship_repository.block_relation(a, b).await?;
                if relation.blocking && !blockers.contains(a) {
                    blockers.push(*a);
                }
                if relation.blocked_by && !blockers.contains(b) {
                    blockers.push(*b);
                }
            }
        }
        Ok(blockers)
    }

    async fn reconcile_with_roster(
        &self,
        call_id: Uuid,
        invitee: &UserId,
        roster: &mut Vec<UserId>,
        request_id: Option<String>,
    ) -> Result<bool, AddCallParticipantsError> {
        let mut index = 0;
        while index < roster.len() {
            let other = roster[index];
            if other == *invitee {
                index += 1;
                continue;
            }
            let relation = self.friendship_repository.block_relation(invitee, &other).await?;
            if relation.blocking {
                return Ok(true);
            }
            if relation.blocked_by {
                super::eviction::evict(
                    self.call_repository.as_ref(),
                    self.voice_service.as_ref(),
                    call_id,
                    &[other],
                    request_id.clone(),
                )
                .await?;
                roster.remove(index);
                continue;
            }
            index += 1;
        }
        Ok(false)
    }

    async fn roster(&self, call_id: Uuid) -> Result<Vec<UserId>, RepositoryError> {
        self.call_repository.active_participant_ids(call_id).await
    }

    async fn blocked_with_roster(
        &self,
        invitee: &UserId,
        roster: &[UserId],
    ) -> Result<bool, FriendshipLookupError> {
        for other in roster.iter().filter(|other| *other != invitee) {
            if self.friendship_repository.is_blocked(invitee, other).await? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn blocked_among(&self, users: &[UserId]) -> Result<bool, FriendshipLookupError> {
        for (index, a) in users.iter().enumerate() {
            for b in &users[index + 1..] {
                if self.friendship_repository.is_blocked(a, b).await? {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    async fn withdraw(&self, call_id: Uuid, invitees: &[UserId], request_id: Option<String>) {
        if let Err(cleanup) = super::eviction::evict(
            self.call_repository.as_ref(),
            self.voice_service.as_ref(),
            call_id,
            invitees,
            request_id,
        )
        .await
        {
            tracing::error!(error = %cleanup, %call_id, "Failed to withdraw invitations after a block lookup error");
        }
    }
}
