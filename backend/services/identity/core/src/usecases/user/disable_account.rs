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
    },
    usecases::auth::reactivation::revoke_access_after_commit,
    usecases::user::activity::clear_observed_activity,
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
    #[error("Activity store error: {0}")]
    ActivityStore(#[from] ActivityStoreError),
}

pub struct DisableAccount<U, W, H, R, S>
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

impl<U, W, H, R, S> DisableAccount<U, W, H, R, S>
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

        let mut tx = self.unit_of_work.begin().await?;
        let snapshot = user;
        let mut user = tx
            .load_user_for_update(&user_id)
            .await?
            .ok_or(DisableAccountError::UserNotFound)?;
        if user.password_hash().as_str() != snapshot.password_hash().as_str()
            || user.updated_at() != snapshot.updated_at()
        {
            self.reauthenticator.verify(&user, reauth).await?;
        }
        let now = Utc::now();
        user.disable(now).map_err(|_| DisableAccountError::UserNotFound)?;
        let mut events: Vec<IdentityEvent> = user.pull_events().into_iter().map(IdentityEvent::from).collect();
        events.push(IdentityEvent::from(UserEvent::ActivityChanged(UserActivityChanged {
            id: EventId::generate(),
            user_id,
            game_id: None,
            name: None,
            started_at: None,
            occurred_at: now,
        })));
        tx.save_user(&user).await?;
        tx.revoke_refresh_tokens_for_user(&user_id, None, now).await?;
        tx.save_events(&events, request_id).await?;
        let observed = self.activity_store.get(&user_id).await?;
        tx.commit().await?;
        if let Err(e) = clear_observed_activity(&*self.activity_store, &user_id, observed.as_ref()).await {
            tracing::warn!(error = %e, user_id = %user_id.uuid(), "Failed to clear the activity cache after disabling the account");
        }
        revoke_access_after_commit(&*self.access_revoker, &user_id, now).await;
        Ok(())
    }
}
