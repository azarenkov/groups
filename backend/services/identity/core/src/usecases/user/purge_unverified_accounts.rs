use std::sync::Arc;

use chrono::{Duration, Utc};
use identity_domain::unit_of_work::IdentityUnitOfWork;

use crate::{
    ports::access_revoker::AccessRevoker,
    usecases::user::account_deletion::{
        AccountDeletionError, DeletionCleanup, apply_account_deletion, clear_deleted_user_caches,
    },
};

const MAX_BATCHES_PER_RUN: usize = 20;

pub struct PurgeUnverifiedAccounts<W, R>
where
    W: IdentityUnitOfWork,
    R: AccessRevoker,
{
    unit_of_work: Arc<W>,
    access_revoker: Arc<R>,
    ttl: Duration,
    batch_size: i64,
}

impl<W, R> PurgeUnverifiedAccounts<W, R>
where
    W: IdentityUnitOfWork,
    R: AccessRevoker,
{
    pub fn new(
        unit_of_work: Arc<W>,
        access_revoker: Arc<R>,
        ttl: Duration,
        batch_size: i64,
    ) -> Self {
        Self {
            unit_of_work,
            access_revoker,
            ttl,
            batch_size,
        }
    }

    pub async fn run_once(&self) -> Result<usize, AccountDeletionError> {
        let mut purged = 0;
        for _ in 0..MAX_BATCHES_PER_RUN {
            let batch = self.purge_batch().await?;
            purged += batch;
            if batch < self.batch_size as usize {
                break;
            }
        }
        Ok(purged)
    }

    async fn purge_batch(&self) -> Result<usize, AccountDeletionError> {
        let now = Utc::now();
        let registered_before = now - self.ttl;
        let mut tx = self.unit_of_work.begin().await?;
        let users = tx
            .load_stale_unverified_users_for_update(registered_before, self.batch_size)
            .await?;
        let mut purged: Vec<DeletionCleanup> = Vec::with_capacity(users.len());
        for mut user in users {
            purged.push(apply_account_deletion(&mut *tx, &mut user, now, None).await?);
        }
        tx.commit().await?;
        self.clear_caches(&purged).await;
        Ok(purged.len())
    }

    async fn clear_caches(&self, purged: &[DeletionCleanup]) {
        for cleanup in purged {
            clear_deleted_user_caches(&*self.unit_of_work, &*self.access_revoker, cleanup).await;
        }
    }
}
