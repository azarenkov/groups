use std::sync::Arc;

use chrono::Utc;
use domain::{
    call::repository::CallRepository,
    shared::repository::error::RepositoryError,
    user::id::UserId,
};

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
    #[error("You can only invite your friends")]
    NotFriends,
    #[error("No recipients specified")]
    NoRecipients,
    #[error("Friendship lookup unavailable: {0}")]
    FriendshipLookup(#[from] FriendshipLookupError),
}

pub struct AddCallParticipants<C, F>
where
    C: CallRepository,
    F: FriendshipLookup,
{
    call_repository: Arc<C>,
    friendship_repository: Arc<F>,
}

impl<C, F> AddCallParticipants<C, F>
where
    C: CallRepository,
    F: FriendshipLookup,
{
    pub fn new(call_repository: Arc<C>, friendship_repository: Arc<F>) -> Self {
        Self {
            call_repository,
            friendship_repository,
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
                .are_friends(&caller_id, &invitee)
                .await?
            {
                return Err(AddCallParticipantsError::NotFriends);
            }
            invitees.push(invitee);
        }
        if invitees.is_empty() {
            return Err(AddCallParticipantsError::NoRecipients);
        }

        self.call_repository
            .add_participants(
                request.call_id,
                &caller_id,
                &invitees,
                Utc::now(),
                request.ctx.request_id,
            )
            .await?;
        Ok(())
    }
}
