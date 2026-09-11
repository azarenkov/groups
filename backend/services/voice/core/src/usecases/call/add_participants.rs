use std::sync::Arc;

use chrono::Utc;
use voice_domain::call::repository::CallRepository;
use domain::user::id::UserId;
use ports::repository::RepositoryError;

use crate::{
    dto::requests::call::add_participants::AddCallParticipantsRequest,
    ports::friendship_lookup::{FriendshipLookup, FriendshipLookupError},
};

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
    #[error("Friendship lookup unavailable: {0}")]
    FriendshipLookup(#[from] FriendshipLookupError),
}

pub struct AddCallParticipants<C, F, U>
where
    C: CallRepository,
    F: FriendshipLookup,
    U: ports::user_profile::UserProfileRepository,
{
    call_repository: Arc<C>,
    friendship_repository: Arc<F>,
    user_repository: Arc<U>,
}

impl<C, F, U> AddCallParticipants<C, F, U>
where
    C: CallRepository,
    F: FriendshipLookup,
    U: ports::user_profile::UserProfileRepository,
{
    pub fn new(
        call_repository: Arc<C>,
        friendship_repository: Arc<F>,
        user_repository: Arc<U>,
    ) -> Self {
        Self {
            call_repository,
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

        let mut invitees: Vec<UserId> = Vec::new();
        for id in &request.invitee_ids {
            let invitee = UserId::new(*id);
            if invitee == caller_id {
                continue;
            }
            if !self
                .friendship_repository
                .can_call(&caller_id, &invitee)
                .await?
            {
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

        let added = self
            .call_repository
            .add_participants(
                request.call_id,
                &caller_id,
                &invitees,
                Utc::now(),
                request.ctx.request_id.clone(),
            )
            .await?;

        let mut remaining = added.len();
        for invitee in &added {
            let blocked = match self.friendship_repository.is_blocked(&caller_id, invitee).await {
                Ok(blocked) => blocked,
                Err(e) => {
                    if let Err(cleanup) = self
                        .call_repository
                        .leave_many(request.call_id, &added, Utc::now(), request.ctx.request_id)
                        .await
                    {
                        tracing::error!(error = %cleanup, call_id = %request.call_id, "Failed to withdraw invitations after a block lookup error");
                    }
                    return Err(e.into());
                }
            };
            if blocked {
                self.call_repository
                    .leave(request.call_id, invitee, Utc::now(), request.ctx.request_id.clone())
                    .await?;
                remaining -= 1;
            }
        }
        if remaining == 0 && !added.is_empty() {
            return Err(AddCallParticipantsError::CallNotAllowed);
        }
        Ok(())
    }
}
