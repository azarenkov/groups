use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{events::MessagingEvent, friendship::FriendshipError, friendship::unit_of_work::FriendshipUnitOfWork};
use domain::user::id::UserId;
use ports::repository::RepositoryError;

use crate::dto::requests::friendship::accept_friend_request::AcceptFriendRequestRequest;

#[derive(Debug, thiserror::Error)]
pub enum AcceptFriendRequestError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Friendship error: {0}")]
    Domain(#[from] FriendshipError),
    #[error("No pending request from this user")]
    NoPendingRequest,
    #[error("This user is blocked")]
    Blocked,
}

pub struct AcceptFriendRequest<W>
where
    W: FriendshipUnitOfWork,
{
    unit_of_work: Arc<W>,
}

impl<W> AcceptFriendRequest<W>
where
    W: FriendshipUnitOfWork,
{
    pub fn new(unit_of_work: Arc<W>) -> Self {
        Self {
            unit_of_work,
        }
    }

    pub async fn execute(
        &self,
        request: AcceptFriendRequestRequest,
        accepter_id: UserId,
    ) -> Result<(), AcceptFriendRequestError> {
        let sender_id = UserId::new(request.sender_id);

        let now = Utc::now();
        let mut tx = self.unit_of_work.begin().await?;
        let existing = tx.load_pair(&accepter_id, &sender_id).await?;
        if tx.block_relation(&accepter_id, &sender_id).await?.any() {
            return Err(AcceptFriendRequestError::Blocked);
        }
        let mut pair = existing.ok_or(AcceptFriendRequestError::NoPendingRequest)?;
        pair.accept(accepter_id, now).map_err(|e| match e {
            FriendshipError::NoPendingRequest => AcceptFriendRequestError::NoPendingRequest,
            other => AcceptFriendRequestError::Domain(other),
        })?;
        let events: Vec<MessagingEvent> = pair.pull_events().into_iter().map(MessagingEvent::from).collect();
        tx.save_pair(&pair).await?;
        tx.save_events(&events, request.ctx.request_id.as_deref()).await?;
        tx.commit().await?;
        Ok(())
    }
}
