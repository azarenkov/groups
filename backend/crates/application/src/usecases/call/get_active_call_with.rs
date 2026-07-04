use std::sync::Arc;

use domain::{
    call::repository::CallRepository, shared::repository::error::RepositoryError, user::id::UserId,
};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum GetActiveCallWithError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct GetActiveCallWith<C>
where
    C: CallRepository,
{
    call_repository: Arc<C>,
}

impl<C> GetActiveCallWith<C>
where
    C: CallRepository,
{
    pub fn new(call_repository: Arc<C>) -> Self {
        Self { call_repository }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        friend_id: Uuid,
    ) -> Result<Option<Uuid>, GetActiveCallWithError> {
        Ok(self
            .call_repository
            .active_call_between(&user_id, &UserId::new(friend_id))
            .await?)
    }
}
