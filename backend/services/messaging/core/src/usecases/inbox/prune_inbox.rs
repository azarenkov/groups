use std::sync::Arc;

use chrono::{DateTime, Utc};
use messaging_domain::inbox::{InboxRepository, InboxRetention};
use ports::repository::RepositoryError;

pub const PRUNE_BATCH: i64 = 500;

pub struct PruneInbox<R: InboxRepository> {
    repository: Arc<R>,
    retention: InboxRetention,
}

impl<R: InboxRepository> PruneInbox<R> {
    pub fn new(repository: Arc<R>, retention: InboxRetention) -> Self {
        Self { repository, retention }
    }

    pub async fn execute(&self, now: DateTime<Utc>) -> Result<u64, RepositoryError> {
        let kept = InboxRetention::KEPT_UNTIL_RESOLVED;
        let before = self.retention.expired_before(now);
        let mut removed = 0;
        loop {
            let batch = self.repository.expire(before, kept, PRUNE_BATCH).await?;
            removed += batch;
            if batch < PRUNE_BATCH as u64 {
                break;
            }
        }
        loop {
            let batch = self.repository.trim(self.retention.max_per_user, kept, PRUNE_BATCH).await?;
            removed += batch;
            if batch < PRUNE_BATCH as u64 {
                break;
            }
        }
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use chrono::Duration;
    use domain::user::id::UserId;
    use messaging_domain::community::invitation::InvitationId;
    use messaging_domain::inbox::{InboxCursor, InboxItem, Notification};
    use uuid::Uuid;

    use super::*;

    #[derive(Default)]
    struct Backlog {
        expired: Mutex<Vec<u64>>,
        overflow: Mutex<Vec<u64>>,
        calls: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl InboxRepository for Backlog {
        async fn deliver(&self, _: &Notification, _: &str) -> Result<bool, RepositoryError> {
            unimplemented!()
        }
        async fn withdraw_invitation(&self, _: &UserId, _: &InvitationId) -> Result<bool, RepositoryError> {
            unimplemented!()
        }
        async fn list(&self, _: &UserId, _: Option<&InboxCursor>, _: i64) -> Result<Vec<InboxItem>, RepositoryError> {
            unimplemented!()
        }
        async fn unread_count(&self, _: &UserId) -> Result<i64, RepositoryError> {
            unimplemented!()
        }
        async fn mark_read(&self, _: &UserId, _: Option<&[Uuid]>, _: DateTime<Utc>) -> Result<(), RepositoryError> {
            unimplemented!()
        }
        async fn expire(&self, before: DateTime<Utc>, kept: &[&str], limit: i64) -> Result<u64, RepositoryError> {
            self.calls.lock().unwrap().push(format!("expire {} {:?} {limit}", before.timestamp(), kept));
            Ok(self.expired.lock().unwrap().pop().unwrap_or(0))
        }
        async fn trim(&self, keep: i64, kept: &[&str], limit: i64) -> Result<u64, RepositoryError> {
            self.calls.lock().unwrap().push(format!("trim {keep} {:?} {limit}", kept));
            Ok(self.overflow.lock().unwrap().pop().unwrap_or(0))
        }
    }

    #[tokio::test]
    async fn drains_expired_then_overflowing_notifications_in_batches() {
        let repository = Arc::new(Backlog::default());
        *repository.expired.lock().unwrap() = vec![7, PRUNE_BATCH as u64];
        *repository.overflow.lock().unwrap() = vec![3];
        let retention = InboxRetention { max_age: Duration::days(30), max_per_user: 200 };
        let now = DateTime::from_timestamp(1_800_000_000, 0).unwrap();
        let removed = PruneInbox::new(repository.clone(), retention).execute(now).await.unwrap();
        assert_eq!(removed, PRUNE_BATCH as u64 + 10);
        let cutoff = (now - Duration::days(30)).timestamp();
        assert_eq!(
            *repository.calls.lock().unwrap(),
            vec![
                format!("expire {cutoff} [\"community_invitation\"] {PRUNE_BATCH}"),
                format!("expire {cutoff} [\"community_invitation\"] {PRUNE_BATCH}"),
                format!("trim 200 [\"community_invitation\"] {PRUNE_BATCH}"),
            ]
        );
    }
}
