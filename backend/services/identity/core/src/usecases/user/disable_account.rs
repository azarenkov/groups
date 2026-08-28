use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use identity_domain::{events::IdentityEvent, unit_of_work::IdentityUnitOfWork, user::repository::UserRepository};
use ports::repository::RepositoryError;

use crate::{
    ports::{
        access_revoker::{AccessRevoker, AccessRevokerError},
        hasher::{HashError, Hasher},
        oauth_verifier::OAuthIdentityVerifier,
    },
    usecases::user::reauth::{Reauth, ReauthError, Reauthenticator},
};

#[derive(Debug, thiserror::Error)]
pub enum DisableAccountError {
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

pub struct DisableAccount<U, W, H, R, V>
where
    U: UserRepository,
    W: IdentityUnitOfWork,
    H: Hasher,
    R: AccessRevoker,
    V: OAuthIdentityVerifier,
{
    user_repository: Arc<U>,
    unit_of_work: Arc<W>,
    access_revoker: Arc<R>,
    reauthenticator: Arc<Reauthenticator<U, H, V>>,
}

impl<U, W, H, R, V> DisableAccount<U, W, H, R, V>
where
    U: UserRepository,
    W: IdentityUnitOfWork,
    H: Hasher,
    R: AccessRevoker,
    V: OAuthIdentityVerifier,
{
    pub fn new(
        user_repository: Arc<U>,
        unit_of_work: Arc<W>,
        access_revoker: Arc<R>,
        reauthenticator: Arc<Reauthenticator<U, H, V>>,
    ) -> Self {
        Self {
            user_repository,
            unit_of_work,
            access_revoker,
            reauthenticator,
        }
    }

    pub async fn execute(&self, user_id: UserId, reauth: &Reauth, request_id: Option<&str>) -> Result<(), DisableAccountError> {
        let user = self
            .user_repository
            .find_by_id(&user_id)
            .await?
            .ok_or(DisableAccountError::UserNotFound)?;
        if user.is_deleted() {
            return Err(DisableAccountError::UserNotFound);
        }
        self.reauthenticator.verify(&user, reauth).await?;

        let now = Utc::now();
        let mut tx = self.unit_of_work.begin().await?;
        let mut user = tx
            .load_user_for_update(&user_id)
            .await?
            .ok_or(DisableAccountError::UserNotFound)?;
        user.disable(now).map_err(|_| DisableAccountError::UserNotFound)?;
        let events: Vec<IdentityEvent> = user.pull_events().into_iter().map(IdentityEvent::from).collect();
        tx.save_user(&user).await?;
        tx.revoke_refresh_tokens_for_user(&user_id, None, now).await?;
        tx.save_events(&events, request_id).await?;
        self.access_revoker.revoke_access_issued_before(&user_id, now).await?;
        tx.commit().await?;
        Ok(())
    }
}
