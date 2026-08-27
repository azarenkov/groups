use std::sync::Arc;

use chrono::Utc;
use voice_domain::call::repository::CallRepository;
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum DeclineCallError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct DeclineCall<C>
where
    C: CallRepository,
{
    call_repository: Arc<C>,
}

impl<C> DeclineCall<C>
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
    ) -> Result<(), DeclineCallError> {
        self.call_repository
            .respond(call_id, &user_id, false, Utc::now(), request_id)
            .await?;
        Ok(())
    }
}
