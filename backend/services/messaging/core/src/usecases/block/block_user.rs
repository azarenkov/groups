use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use messaging_domain::{events::MessagingEvent, friendship::FriendshipError, friendship::events::FriendshipDomainEvent, friendship::events::lifecycle::UserBlocked, friendship::unit_of_work::FriendshipUnitOfWork};
use ports::{repository::RepositoryError, user_profile::UserProfileRepository};

#[derive(Debug, thiserror::Error)]
pub enum BlockUserError {
    #[error("Cannot block yourself")]
    SelfBlock,
    #[error("User not found")]
    UserNotFound,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Friendship error: {0}")]
    Domain(#[from] FriendshipError),
}

pub struct BlockUser<W: FriendshipUnitOfWork, U: UserProfileRepository> {
    unit_of_work: Arc<W>,
    user_repository: Arc<U>,
}

impl<W: FriendshipUnitOfWork, U: UserProfileRepository> BlockUser<W, U> {
    pub fn new(unit_of_work: Arc<W>, user_repository: Arc<U>) -> Self {
        Self {
            unit_of_work,
            user_repository,
        }
    }

    pub async fn execute(&self, blocker_id: UserId, blocked_id: UserId) -> Result<(), BlockUserError> {
        if blocker_id == blocked_id {
            return Err(BlockUserError::SelfBlock);
        }
        self.user_repository
            .get_name(&blocked_id)
            .await?
            .ok_or(BlockUserError::UserNotFound)?;

        let now = Utc::now();
        let mut tx = self.unit_of_work.begin().await?;
        let mut events: Vec<MessagingEvent> = Vec::new();
        if let Some(mut pair) = tx.load_pair(&blocker_id, &blocked_id).await? {
            let removed = pair.remove(blocker_id, now).is_ok();
            if removed {
                events.extend(pair.pull_events().into_iter().map(MessagingEvent::from));
                tx.save_pair(&pair).await?;
            }
        }
        if tx.insert_block(&blocker_id, &blocked_id, now).await? {
            events.push(MessagingEvent::from(FriendshipDomainEvent::Blocked(
                UserBlocked::record(blocker_id, blocked_id, now),
            )));
        }
        tx.save_events(&events, None).await?;
        tx.commit().await?;
        Ok(())
    }
}
