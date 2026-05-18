use std::sync::Arc;

use chrono::Utc;
use domain::{
    friendship::{Friendship, repository::FriendshipRepository, status::FriendshipStatus},
    shared::{
        outbox::{
            outbox_record::OutboxRecord,
            to_outbox::{OutboxError, ToOutboxRecord},
        },
        repository::error::RepositoryError,
    },
    user::id::UserId,
};

use crate::dto::requests::friendship::remove_friendship::RemoveFriendshipRequest;

#[derive(Debug, thiserror::Error)]
pub enum RemoveFriendshipError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
    #[error("Friendship not found")]
    NotFound,
}

pub struct RemoveFriendship<F>
where
    F: FriendshipRepository,
{
    friendship_repository: Arc<F>,
}

impl<F> RemoveFriendship<F>
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
        let records: Vec<OutboxRecord> = events
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
            .collect::<Result<_, _>>()?;

        let success = match existing.status {
            FriendshipStatus::PendingIncoming => {
                self.friendship_repository
                    .decline_request_with_events(&requester_id, &other_id, &records)
                    .await?
            }
            _ => {
                self.friendship_repository
                    .delete_pair_with_events(&requester_id, &other_id, &records)
                    .await?
            }
        };

        if !success {
            return Err(RemoveFriendshipError::NotFound);
        }
        Ok(())
    }
}
