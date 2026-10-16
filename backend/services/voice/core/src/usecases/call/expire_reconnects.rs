use std::sync::Arc;

use chrono::{DateTime, Utc};
use ports::repository::RepositoryError;
use voice_domain::call::repository::CallRepository;

const EXPIRE_BATCH: i64 = 100;

#[derive(Debug, thiserror::Error)]
pub enum ExpireCallReconnectsError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct ExpireCallReconnects<C>
where
    C: CallRepository,
{
    call_repository: Arc<C>,
}

impl<C> ExpireCallReconnects<C>
where
    C: CallRepository,
{
    pub fn new(call_repository: Arc<C>) -> Self {
        Self { call_repository }
    }

    pub async fn execute(&self, now: DateTime<Utc>) -> Result<usize, ExpireCallReconnectsError> {
        let mut total = 0;
        loop {
            let expired = self.call_repository.expire_reconnects(now, EXPIRE_BATCH, None).await?;
            total += expired;
            if expired < EXPIRE_BATCH as usize {
                return Ok(total);
            }
        }
    }
}
