use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use ports::repository::RepositoryError;

use crate::ports::call_log_retention::CallLogRetention;

pub const CALL_LOG_RETENTION_DAYS: i64 = 400;
pub const PRUNE_BATCH: i64 = 500;

pub struct PruneCallLog<R: CallLogRetention> {
    retention: Arc<R>,
}

impl<R: CallLogRetention> PruneCallLog<R> {
    pub fn new(retention: Arc<R>) -> Self {
        Self { retention }
    }

    pub async fn execute(&self, now: DateTime<Utc>) -> Result<u64, RepositoryError> {
        let before = now - Duration::days(CALL_LOG_RETENTION_DAYS);
        let mut removed = 0;
        loop {
            let batch = self.retention.forget_consumed_before(before, PRUNE_BATCH).await?;
            removed += batch;
            if batch < PRUNE_BATCH as u64 {
                return Ok(removed);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;

    use super::*;

    #[derive(Default)]
    struct Consumed {
        batches: Mutex<Vec<u64>>,
        cutoffs: Mutex<Vec<DateTime<Utc>>>,
    }

    #[async_trait]
    impl CallLogRetention for Consumed {
        async fn forget_consumed_before(&self, before: DateTime<Utc>, _: i64) -> Result<u64, RepositoryError> {
            self.cutoffs.lock().unwrap().push(before);
            Ok(self.batches.lock().unwrap().pop().unwrap_or(0))
        }
    }

    #[tokio::test]
    async fn forgets_consumed_calls_older_than_the_retention_in_batches() {
        let consumed = Arc::new(Consumed::default());
        *consumed.batches.lock().unwrap() = vec![3, PRUNE_BATCH as u64];
        let now = DateTime::from_timestamp(1_800_000_000, 0).unwrap();
        let removed = PruneCallLog::new(consumed.clone()).execute(now).await.unwrap();
        assert_eq!(removed, PRUNE_BATCH as u64 + 3);
        let cutoffs = consumed.cutoffs.lock().unwrap();
        assert_eq!(cutoffs.len(), 2);
        assert_eq!(cutoffs[0], now - Duration::days(CALL_LOG_RETENTION_DAYS));
    }
}
