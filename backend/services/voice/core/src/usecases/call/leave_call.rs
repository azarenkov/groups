use std::sync::Arc;

use chrono::Utc;
use voice_domain::call::repository::CallRepository;
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum LeaveCallError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct LeaveCall<C>
where
    C: CallRepository,
{
    call_repository: Arc<C>,
}

impl<C> LeaveCall<C>
where
    C: CallRepository,
{
    pub fn new(call_repository: Arc<C>) -> Self {
        Self { call_repository }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        call_id: Uuid,
        request_id: Option<String>,
    ) -> Result<(), LeaveCallError> {
        self.call_repository
            .leave(call_id, &user_id, Utc::now(), request_id)
            .await?;
        Ok(())
    }
}
