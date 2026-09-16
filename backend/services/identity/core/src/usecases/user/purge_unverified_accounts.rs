use std::sync::Arc;

use chrono::{Duration, Utc};
use identity_domain::unit_of_work::IdentityUnitOfWork;

use crate::{
    ports::{access_revoker::AccessRevoker, activity_store::ActivityStore},
    usecases::user::account_deletion::{AccountDeletionError, SideEffects, apply_account_deletion},
};

const MAX_BATCHES_PER_RUN: usize = 20;

pub struct PurgeUnverifiedAccounts<W, R, S>
where
    W: IdentityUnitOfWork,
    R: AccessRevoker,
    S: ActivityStore,
{
    unit_of_work: Arc<W>,
    access_revoker: Arc<R>,
    activity_store: Arc<S>,
    ttl: Duration,
    batch_size: i64,
}

impl<W, R, S> PurgeUnverifiedAccounts<W, R, S>
where
    W: IdentityUnitOfWork,
    R: AccessRevoker,
    S: ActivityStore,
{
    pub fn new(
        unit_of_work: Arc<W>,
        access_revoker: Arc<R>,
        activity_store: Arc<S>,
        ttl: Duration,
        batch_size: i64,
    ) -> Self {
        Self {
            unit_of_work,
            access_revoker,
            activity_store,
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
        let purged = users.len();
        for mut user in users {
            apply_account_deletion(
                &mut *tx,
                &mut user,
                now,
                &*self.activity_store,
                &*self.access_revoker,
                SideEffects::BestEffort,
                None,
            )
            .await?;
        }
        tx.commit().await?;
        Ok(purged)
    }
}
