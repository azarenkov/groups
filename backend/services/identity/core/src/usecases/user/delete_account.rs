use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use identity_domain::{events::IdentityEvent, unit_of_work::IdentityUnitOfWork, user::repository::UserRepository};
use ports::repository::RepositoryError;

use crate::ports::{
    access_revoker::{AccessRevoker, AccessRevokerError},
    hasher::{HashError, Hasher},
};

#[derive(Debug, thiserror::Error)]
pub enum DeleteAccountError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Hash error: {0}")]
    Hash(#[from] HashError),
    #[error("Access revocation error: {0}")]
    AccessRevoker(#[from] AccessRevokerError),
    #[error("Password is incorrect")]
    InvalidPassword,
    #[error("User not found")]
    UserNotFound,
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
    hasher: Arc<H>,
    access_revoker: Arc<R>,
}

impl<U, W, H, R> DeleteAccount<U, W, H, R>
where
    U: UserRepository,
    W: IdentityUnitOfWork,
    H: Hasher,
    R: AccessRevoker,
{
    pub fn new(user_repository: Arc<U>, unit_of_work: Arc<W>, hasher: Arc<H>, access_revoker: Arc<R>) -> Self {
        Self {
            user_repository,
            unit_of_work,
            hasher,
            access_revoker,
        }
    }

    pub async fn execute(&self, user_id: UserId, password: &str, request_id: Option<&str>) -> Result<(), DeleteAccountError> {
        let user = self
            .user_repository
            .find_by_id(&user_id)
            .await?
            .ok_or(DeleteAccountError::UserNotFound)?;
        if user.is_deleted() {
            return Err(DeleteAccountError::UserNotFound);
        }
        if !self.hasher.verify(password, user.password_hash().as_str())? {
            return Err(DeleteAccountError::InvalidPassword);
        }

        let now = Utc::now();
        let mut tx = self.unit_of_work.begin().await?;
        let mut user = tx
            .load_user_for_update(&user_id)
            .await?
            .ok_or(DeleteAccountError::UserNotFound)?;
        user.delete(now).map_err(|_| DeleteAccountError::UserNotFound)?;
        let events: Vec<IdentityEvent> = user.pull_events().into_iter().map(IdentityEvent::from).collect();
        tx.save_user(&user).await?;
        tx.delete_oauth_identities(&user_id).await?;
        tx.revoke_refresh_tokens_for_user(&user_id, None, now).await?;
        tx.save_events(&events, request_id).await?;
        tx.commit().await?;
        self.access_revoker.revoke_access_issued_before(&user_id, now).await?;
        Ok(())
    }
}
