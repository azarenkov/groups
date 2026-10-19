use std::sync::Arc;

use identity_domain::unit_of_work::IdentityUnitOfWork;
use ports::repository::RepositoryError;

use crate::{
    ports::access_revoker::{AccessRevoker, AccessRevokerError},
    usecases::auth::reactivation::clear_recorded_revocation,
};

#[derive(Debug, thiserror::Error)]
pub enum ReapplyAccessRevocationsError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Access revocation error: {0}")]
    AccessRevoker(#[from] AccessRevokerError),
}

pub struct ReapplyAccessRevocations<W: IdentityUnitOfWork, R: AccessRevoker> {
    unit_of_work: Arc<W>,
    access_revoker: Arc<R>,
    batch_size: i64,
}

impl<W: IdentityUnitOfWork, R: AccessRevoker> ReapplyAccessRevocations<W, R> {
    pub fn new(unit_of_work: Arc<W>, access_revoker: Arc<R>, batch_size: i64) -> Self {
        Self {
            unit_of_work,
            access_revoker,
            batch_size,
        }
    }

    pub async fn run_once(&self) -> Result<usize, ReapplyAccessRevocationsError> {
        let pending = {
            let mut tx = self.unit_of_work.begin().await?;
            let pending = tx.load_pending_access_revocations(self.batch_size).await?;
            tx.commit().await?;
            pending
        };
        let mut applied = 0;
        for revocation in &pending {
            self.access_revoker
                .revoke_access_issued_before(revocation.user_id(), revocation.cutoff())
                .await?;
            clear_recorded_revocation(&*self.unit_of_work, revocation).await?;
            applied += 1;
        }
        Ok(applied)
    }
}
