use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use identity_domain::{unit_of_work::IdentityUnitOfWork, user::repository::UserRepository};
use ports::repository::RepositoryError;

use crate::{
    ports::{
        access_revoker::{AccessRevoker, AccessRevokerError},
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
}

impl From<AccountDeletionError> for DeleteAccountError {
    fn from(error: AccountDeletionError) -> Self {
        match error {
            AccountDeletionError::Repository(e) => Self::Repository(e),
            AccountDeletionError::AlreadyDeleted => Self::UserNotFound,
        }
    }
}

pub struct DeleteAccount<U, W, H, R>
where
    U: UserRepository,
    W: IdentityUnitOfWork,
    H: Hasher,
    R: AccessRevoker,
{
    user_repository: Arc<U>,
    unit_of_work: Arc<W>,
    access_revoker: Arc<R>,
    reauthenticator: Arc<Reauthenticator<H>>,
}

impl<U, W, H, R> DeleteAccount<U, W, H, R>
where
    U: UserRepository,
    W: IdentityUnitOfWork,
    H: Hasher,
    R: AccessRevoker,
{
    pub fn new(
        user_repository: Arc<U>,
        unit_of_work: Arc<W>,
        access_revoker: Arc<R>,
        reauthenticator: Arc<Reauthenticator<H>>,
    ) -> Self {
        Self {
            user_repository,
            unit_of_work,
            access_revoker,
            reauthenticator,
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
        let cleanup = apply_account_deletion(&mut *tx, &mut user, now, request_id).await?;
        tx.commit().await?;
        clear_deleted_user_caches(&*self.unit_of_work, &*self.access_revoker, &cleanup).await;
        Ok(())
    }
}
