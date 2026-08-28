use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{events::MessagingEvent, friendship::unit_of_work::FriendshipUnitOfWork};
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

pub struct RemoveFriendship<W>
where
    W: FriendshipUnitOfWork,
{
    unit_of_work: Arc<W>,
}

impl<W> RemoveFriendship<W>
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
        request: RemoveFriendshipRequest,
        requester_id: UserId,
    ) -> Result<(), RemoveFriendshipError> {
        let other_id = UserId::new(request.other_user_id);

        let now = Utc::now();
        let mut tx = self.unit_of_work.begin().await?;
        let mut pair = tx
            .load_pair(&requester_id, &other_id)
            .await?
            .ok_or(RemoveFriendshipError::NotFound)?;
        pair.remove(requester_id, now)
            .map_err(|_| RemoveFriendshipError::NotFound)?;
        let events: Vec<MessagingEvent> = pair.pull_events().into_iter().map(MessagingEvent::from).collect();
        tx.save_pair(&pair).await?;
        tx.save_events(&events, request.ctx.request_id.as_deref()).await?;
        tx.commit().await?;
        Ok(())
    }
}
