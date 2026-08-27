use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{events::MessagingEvent, friendship::Friendship, friendship::repository::FriendshipRepository, friendship::status::FriendshipStatus, friendship::unit_of_work::FriendshipUnitOfWork};
use domain::user::id::UserId;
use ports::repository::RepositoryError;

use crate::dto::requests::friendship::remove_friendship::RemoveFriendshipRequest;

#[derive(Debug, thiserror::Error)]
pub enum RemoveFriendshipError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Friendship not found")]
    NotFound,
}

pub struct RemoveFriendship<F, W>
where
    F: FriendshipRepository,
    W: FriendshipUnitOfWork,
{
    friendship_repository: Arc<F>,
    unit_of_work: Arc<W>,
}

impl<F, W> RemoveFriendship<F, W>
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
        request: RemoveFriendshipRequest,
        requester_id: UserId,
    ) -> Result<(), RemoveFriendshipError> {
        let other_id = UserId::new(request.other_user_id);

        let existing = self
            .friendship_repository
            .find_pair(&requester_id, &other_id)
            .await?
            .ok_or(RemoveFriendshipError::NotFound)?;

        let now = Utc::now();
        let events = Friendship::removal_events(requester_id, other_id, now);

        let request_id = request.ctx.request_id.clone();
        
        let events: Vec<MessagingEvent> = events.into_iter().map(MessagingEvent::from).collect();
        let mut tx = self.unit_of_work.begin().await?;
        let success = match existing.status {
            FriendshipStatus::PendingIncoming => {
                tx.decline_request(&requester_id, &other_id).await?
            }
            _ => tx.delete_pair(&requester_id, &other_id).await?,
        };
        if !success {
            return Err(RemoveFriendshipError::NotFound);
        }
        tx.save_events(&events, request_id.as_deref()).await?;
        tx.commit().await?;
        Ok(())
    }
}
