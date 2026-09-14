use std::sync::Arc;

use voice_domain::call::repository::CallRepository;
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use uuid::Uuid;

use crate::ports::friendship_lookup::{FriendshipLookup, FriendshipLookupError};

#[derive(Debug, thiserror::Error)]
pub enum GetActiveCallWithError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Friendship lookup unavailable: {0}")]
    FriendshipLookup(#[from] FriendshipLookupError),
}

pub struct GetActiveCallWith<C, F>
where
    C: CallRepository,
    F: FriendshipLookup,
{
    call_repository: Arc<C>,
    friendship_repository: Arc<F>,
}

impl<C, F> GetActiveCallWith<C, F>
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
        user_id: UserId,
        friend_id: Uuid,
    ) -> Result<Option<Uuid>, GetActiveCallWithError> {
        let friend_id = UserId::new(friend_id);
        if self
            .friendship_repository
            .is_blocked(&user_id, &friend_id)
            .await?
        {
            return Ok(None);
        }
        Ok(self
            .call_repository
            .active_call_between(&user_id, &friend_id)
            .await?)
    }
}
