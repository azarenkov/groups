use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{friendship::Friendship, friendship::FriendshipError, friendship::repository::FriendshipRepository, friendship::status::FriendshipStatus};
use domain::{shared::outbox::outbox_record::OutboxRecord, shared::outbox::to_outbox::OutboxError, shared::outbox::to_outbox::ToOutboxRecord, shared::repository::error::RepositoryError, user::id::UserId};

use crate::dto::requests::friendship::accept_friend_request::AcceptFriendRequestRequest;

#[derive(Debug, thiserror::Error)]
pub enum AcceptFriendRequestError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Friendship error: {0}")]
    Domain(#[from] FriendshipError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
    #[error("No pending request from this user")]
    NoPendingRequest,
}

pub struct AcceptFriendRequest<F>
where
    F: FriendshipRepository,
{
    friendship_repository: Arc<F>,
}

impl<F> AcceptFriendRequest<F>
where
    F: FriendshipRepository,
{
    pub fn new(friendship_repository: Arc<F>) -> Self {
        Self {
            friendship_repository,
        }
    }

    pub async fn execute(
        &self,
        request: AcceptFriendRequestRequest,
        accepter_id: UserId,
    ) -> Result<(), AcceptFriendRequestError> {
        let sender_id = UserId::new(request.sender_id);

        let existing = self
            .friendship_repository
            .find_pair(&accepter_id, &sender_id)
            .await?
            .ok_or(AcceptFriendRequestError::NoPendingRequest)?;
        if existing.status != FriendshipStatus::PendingIncoming {
            return Err(AcceptFriendRequestError::NoPendingRequest);
        }

        let now = Utc::now();
        let events = Friendship::accept_pair(accepter_id, sender_id, now);

        let request_id = request.ctx.request_id.clone();
        let records: Vec<OutboxRecord> = events
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
            .collect::<Result<_, _>>()?;

        let updated = self
            .friendship_repository
            .accept_pair_with_events(&accepter_id, &sender_id, &records)
            .await?;
        if !updated {
            return Err(AcceptFriendRequestError::NoPendingRequest);
        }
        Ok(())
    }
}
