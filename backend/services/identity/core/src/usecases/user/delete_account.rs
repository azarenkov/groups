use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use domain::event::EventId;
use identity_domain::{
    events::IdentityEvent, unit_of_work::IdentityUnitOfWork, user::events::UserEvent,
    user::events::lifecycle::UserActivityChanged, user::repository::UserRepository,
};
use ports::repository::RepositoryError;

use crate::{
    ports::{
        access_revoker::{AccessRevoker, AccessRevokerError},
        activity_store::{ActivityStore, ActivityStoreError},
        hasher::{HashError, Hasher},
        oauth_verifier::OAuthIdentityVerifier,
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

pub struct DeleteAccount<U, W, H, R, V, S>
where
    U: UserRepository,
    W: IdentityUnitOfWork,
    H: Hasher,
    R: AccessRevoker,
    V: OAuthIdentityVerifier,
    S: ActivityStore,
{
    user_repository: Arc<U>,
    unit_of_work: Arc<W>,
    access_revoker: Arc<R>,
    reauthenticator: Arc<Reauthenticator<U, H, V>>,
    activity_store: Arc<S>,
}

impl<U, W, H, R, V, S> DeleteAccount<U, W, H, R, V, S>
where
    U: UserRepository,
    W: IdentityUnitOfWork,
    H: Hasher,
    R: AccessRevoker,
    V: OAuthIdentityVerifier,
    S: ActivityStore,
{
    pub fn new(
        user_repository: Arc<U>,
        unit_of_work: Arc<W>,
        access_revoker: Arc<R>,
        reauthenticator: Arc<Reauthenticator<U, H, V>>,
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
        user.delete(now).map_err(|_| DeleteAccountError::UserNotFound)?;
        let mut events: Vec<IdentityEvent> = user.pull_events().into_iter().map(IdentityEvent::from).collect();
        self.activity_store.clear(&user_id).await?;
        events.push(IdentityEvent::from(UserEvent::ActivityChanged(UserActivityChanged {
            id: EventId::generate(),
            user_id,
            game_id: None,
            name: None,
            started_at: None,
            occurred_at: now,
        })));
        tx.save_user(&user).await?;
        tx.delete_oauth_identities(&user_id).await?;
        tx.revoke_refresh_tokens_for_user(&user_id, None, now).await?;
        tx.save_events(&events, request_id).await?;
        self.access_revoker.revoke_access_issued_before(&user_id, now).await?;
        tx.commit().await?;
        Ok(())
    }
}
