use std::sync::Arc;

use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use messaging_domain::{events::MessagingEvent, friendship::events::FriendshipDomainEvent, friendship::events::lifecycle::UserUnblocked, friendship::unit_of_work::FriendshipUnitOfWork};
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum UnblockUserError {
    #[error("User is not blocked")]
    NotBlocked,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct UnblockUser<W: FriendshipUnitOfWork> {
    unit_of_work: Arc<W>,
}

impl<W: FriendshipUnitOfWork> UnblockUser<W> {
    pub fn new(unit_of_work: Arc<W>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        blocker_id: UserId,
        blocked_id: UserId,
    ) -> Result<DateTime<Utc>, UnblockUserError> {
        let now = Utc::now();
        let mut tx = self.unit_of_work.begin().await?;
        if !tx.delete_block(&blocker_id, &blocked_id).await? {
            return Err(UnblockUserError::NotBlocked);
        }
        let events = vec![MessagingEvent::from(FriendshipDomainEvent::Unblocked(
            UserUnblocked::record(blocker_id, blocked_id, now),
        ))];
        tx.save_events(&events, None).await?;
        tx.commit().await?;
        Ok(now)
    }
}
