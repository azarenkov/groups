use std::sync::Arc;

use messaging_domain::community::invitation::LegacyInvitations;
use ports::repository::RepositoryError;

pub const ADOPT_BATCH: i64 = 500;

pub struct AdoptLegacyInvitations<L: LegacyInvitations> {
    legacy: Arc<L>,
}

impl<L: LegacyInvitations> AdoptLegacyInvitations<L> {
    pub fn new(legacy: Arc<L>) -> Self {
        Self { legacy }
    }

    pub async fn execute(&self) -> Result<u64, RepositoryError> {
        let mut adopted = 0;
        loop {
            let batch = self.legacy.adopt(ADOPT_BATCH).await?;
            adopted += batch;
            if batch < ADOPT_BATCH as u64 {
                break;
            }
        }
        loop {
            if self.legacy.release_blocked(ADOPT_BATCH).await? < ADOPT_BATCH as u64 {
                break;
            }
        }
        loop {
            if self.legacy.announce_removed(ADOPT_BATCH).await? < ADOPT_BATCH as u64 {
                return Ok(adopted);
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
    struct Backlog {
        legacy: Mutex<Vec<u64>>,
        blocked: Mutex<Vec<u64>>,
        removed: Mutex<Vec<u64>>,
    }

    #[async_trait]
    impl LegacyInvitations for Backlog {
        async fn adopt(&self, limit: i64) -> Result<u64, RepositoryError> {
            assert_eq!(limit, ADOPT_BATCH);
            Ok(self.legacy.lock().unwrap().pop().unwrap_or(0))
        }
        async fn release_blocked(&self, limit: i64) -> Result<u64, RepositoryError> {
            assert_eq!(limit, ADOPT_BATCH);
            Ok(self.blocked.lock().unwrap().pop().unwrap_or(0))
        }
        async fn announce_removed(&self, limit: i64) -> Result<u64, RepositoryError> {
            assert_eq!(limit, ADOPT_BATCH);
            Ok(self.removed.lock().unwrap().pop().unwrap_or(0))
        }
    }

    #[tokio::test]
    async fn drains_legacy_invitations_releases_blocked_pairs_and_announces_removed_messages() {
        let backlog = Arc::new(Backlog::default());
        *backlog.legacy.lock().unwrap() = vec![4, ADOPT_BATCH as u64, ADOPT_BATCH as u64];
        *backlog.blocked.lock().unwrap() = vec![2, ADOPT_BATCH as u64];
        *backlog.removed.lock().unwrap() = vec![7, ADOPT_BATCH as u64];
        let adopted = AdoptLegacyInvitations::new(backlog.clone()).execute().await.unwrap();
        assert_eq!(adopted, 2 * ADOPT_BATCH as u64 + 4);
        assert!(backlog.legacy.lock().unwrap().is_empty());
        assert!(backlog.blocked.lock().unwrap().is_empty());
        assert!(backlog.removed.lock().unwrap().is_empty());
    }
}
