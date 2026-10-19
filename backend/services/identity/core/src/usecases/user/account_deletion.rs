use chrono::{DateTime, Utc};
use identity_domain::{
    events::IdentityEvent, unit_of_work::IdentityTransaction, unit_of_work::IdentityUnitOfWork, user::User,
    user::access_revocation::AccessRevocation,
};
use ports::repository::RepositoryError;

use crate::{ports::access_revoker::AccessRevoker, usecases::auth::reactivation::revoke_access_after_commit};

#[derive(Debug, thiserror::Error)]
pub enum AccountDeletionError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Account is already deleted")]
    AlreadyDeleted,
}

pub async fn apply_account_deletion(
    tx: &mut dyn IdentityTransaction,
    user: &mut User,
    now: DateTime<Utc>,
    request_id: Option<&str>,
) -> Result<DeletionCleanup, AccountDeletionError> {
    let user_id = user.id_ownned();
    user.delete(now)
        .map_err(|_| AccountDeletionError::AlreadyDeleted)?;
    let events: Vec<IdentityEvent> =
        user.pull_events().into_iter().map(IdentityEvent::from).collect();
    tx.save_user(user).await?;
    tx.revoke_refresh_tokens_for_user(&user_id, None, now).await?;
    let revocation = AccessRevocation::new(user_id, now);
    tx.record_access_revocation(&revocation).await?;
    tx.save_events(&events, request_id).await?;
    Ok(DeletionCleanup { revocation })
}

pub struct DeletionCleanup {
    revocation: AccessRevocation,
}

pub async fn clear_deleted_user_caches<W, R>(
    unit_of_work: &W,
    access_revoker: &R,
    cleanup: &DeletionCleanup,
)
where
    W: IdentityUnitOfWork + ?Sized,
    R: AccessRevoker + ?Sized,
{
    revoke_access_after_commit(unit_of_work, access_revoker, &cleanup.revocation).await;
}
