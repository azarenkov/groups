use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{friendship::Friendship, friendship::FriendshipError, friendship::repository::FriendshipRepository, friendship::status::FriendshipStatus, friendship::unit_of_work::FriendshipUnitOfWork};
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

pub struct AcceptFriendRequest<F, W>
where
    F: FriendshipRepository,
    W: FriendshipUnitOfWork,
{
    friendship_repository: Arc<F>,
    unit_of_work: Arc<W>,
}

impl<F, W> AcceptFriendRequest<F, W>
where
    F: FriendshipRepository,
    W: FriendshipUnitOfWork,
{
    pub fn new(friendship_repository: Arc<F>, unit_of_work: Arc<W>) -> Self {
        Self {
            friendship_repository,
            unit_of_work,
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

        let mut tx = self.unit_of_work.begin().await?;
        if !tx.accept_pair(&accepter_id, &sender_id).await? {
            return Err(AcceptFriendRequestError::NoPendingRequest);
        }
        tx.save_outbox(&records).await?;
        tx.commit().await?;
        Ok(())
    }
}
