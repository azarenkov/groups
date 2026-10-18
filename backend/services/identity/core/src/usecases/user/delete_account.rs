use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use identity_domain::{unit_of_work::IdentityUnitOfWork, user::repository::UserRepository};
use ports::repository::RepositoryError;

use crate::{
    ports::{
        access_revoker::{AccessRevoker, AccessRevokerError},
        activity_store::{ActivityStore, ActivityStoreError},
        hasher::{HashError, Hasher},
    },
    usecases::user::account_deletion::{
        AccountDeletionError, apply_account_deletion, clear_deleted_user_caches,
    },
    usecases::user::reauth::{Reauth, ReauthError, Reauthenticator},
};

#[derive(Debug, thiserror::Error)]
pub enum DeleteAccountError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Hash error: {0}")]
    Hash(#[from] HashError),
    #[error("Access revocation error: {0}")]
    AccessRevoker(#[from] AccessRevokerError),
    #[error("Re-authentication failed: {0}")]
    Reauth(#[from] ReauthError),
    #[error("User not found")]
    UserNotFound,
    #[error("Activity store error: {0}")]
    ActivityStore(#[from] ActivityStoreError),
}

impl From<AccountDeletionError> for DeleteAccountError {
    fn from(error: AccountDeletionError) -> Self {
        match error {
            AccountDeletionError::Repository(e) => Self::Repository(e),
            AccountDeletionError::AccessRevoker(e) => Self::AccessRevoker(e),
            AccountDeletionError::ActivityStore(e) => Self::ActivityStore(e),
            AccountDeletionError::AlreadyDeleted => Self::UserNotFound,
        }
    }
}

pub struct DeleteAccount<U, W, H, R, S>
where
    U: UserRepository,
    W: IdentityUnitOfWork,
    H: Hasher,
    R: AccessRevoker,
    S: ActivityStore,
{
    user_repository: Arc<U>,
    unit_of_work: Arc<W>,
    access_revoker: Arc<R>,
    reauthenticator: Arc<Reauthenticator<H>>,
    activity_store: Arc<S>,
}

impl<U, W, H, R, S> DeleteAccount<U, W, H, R, S>
where
    U: UserRepository,
    W: IdentityUnitOfWork,
    H: Hasher,
    R: AccessRevoker,
    S: ActivityStore,
{
    pub fn new(
        user_repository: Arc<U>,
        unit_of_work: Arc<W>,
        access_revoker: Arc<R>,
        reauthenticator: Arc<Reauthenticator<H>>,
        activity_store: Arc<S>,
    ) -> Self {
        Self {
            user_repository,
            unit_of_work,
            access_revoker,
            reauthenticator,
            activity_store,
        }
    }

    pub async fn execute(&self, user_id: UserId, reauth: &Reauth, request_id: Option<&str>) -> Result<(), DeleteAccountError> {
        let user = self
            .user_repository
            .find_by_id(&user_id)
            .await?
            .ok_or(DeleteAccountError::UserNotFound)?;
        if user.is_deleted() {
            return Err(DeleteAccountError::UserNotFound);
        }
        self.reauthenticator.verify(&user, reauth).await?;

        let mut tx = self.unit_of_work.begin().await?;
        let snapshot = user;
        let mut user = tx
            .load_user_for_update(&user_id)
            .await?
            .ok_or(DeleteAccountError::UserNotFound)?;
        if user.password_hash().as_str() != snapshot.password_hash().as_str()
            || user.updated_at() != snapshot.updated_at()
        {
            self.reauthenticator.verify(&user, reauth).await?;
        }
        let now = Utc::now();
        apply_account_deletion(&mut *tx, &mut user, now, request_id).await?;
        tx.commit().await?;
        if let Err(e) = clear_deleted_user_caches(&*self.activity_store, &*self.access_revoker, &user_id, now).await {
            tracing::warn!(error = %e, user_id = %user_id.uuid(), "Failed to clear caches after deleting the account");
        }
        Ok(())
    }
}
