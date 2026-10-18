use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use identity_domain::{events::IdentityEvent, unit_of_work::IdentityUnitOfWork, user::presence::PresencePreference};
use ports::repository::RepositoryError;

use crate::ports::activity_store::{ActivityStore, ActivityStoreError};

#[derive(Debug, thiserror::Error)]
pub enum PresencePreferenceError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("User not found")]
    UserNotFound,
}

#[derive(Debug, thiserror::Error)]
pub enum ImportLegacyPreferencesError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Legacy store error: {0}")]
    LegacyStore(#[from] ActivityStoreError),
}

pub struct SetPresencePreference<W: IdentityUnitOfWork> {
    unit_of_work: Arc<W>,
}

impl<W: IdentityUnitOfWork> SetPresencePreference<W> {
    pub fn new(unit_of_work: Arc<W>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        appear_offline: bool,
        request_id: Option<&str>,
    ) -> Result<bool, PresencePreferenceError> {
        let mut tx = self.unit_of_work.begin().await?;
        if !tx.lock_user(&user_id).await? {
            return Err(PresencePreferenceError::UserNotFound);
        }
        let now = Utc::now();
        let mut preference = tx
            .load_presence_preference_for_update(&user_id)
            .await?
            .unwrap_or_else(|| PresencePreference::visible(user_id, now));
        if let Some(event) = preference.set_appear_offline(appear_offline, now) {
            tx.save_presence_preference(&preference).await?;
            tx.save_events(&[IdentityEvent::from(event)], request_id).await?;
        }
        tx.commit().await?;
        Ok(preference.appear_offline())
    }
}

pub struct ImportLegacyPresencePreferences<S: ActivityStore, W: IdentityUnitOfWork> {
    store: Arc<S>,
    unit_of_work: Arc<W>,
}

impl<S: ActivityStore, W: IdentityUnitOfWork> ImportLegacyPresencePreferences<S, W> {
    pub fn new(store: Arc<S>, unit_of_work: Arc<W>) -> Self {
        Self { store, unit_of_work }
    }

    pub async fn execute(&self) -> Result<usize, ImportLegacyPreferencesError> {
        let mut imported = 0;
        for user_id in self.store.legacy_hidden_users().await? {
            let mut tx = self.unit_of_work.begin().await?;
            if tx.lock_user(&user_id).await? {
                let now = Utc::now();
                let mut preference = tx
                    .load_presence_preference_for_update(&user_id)
                    .await?
                    .unwrap_or_else(|| PresencePreference::visible(user_id, now));
                if let Some(event) = preference.set_appear_offline(true, now) {
                    tx.save_presence_preference(&preference).await?;
                    tx.save_events(&[IdentityEvent::from(event)], None).await?;
                    imported += 1;
                }
            }
            tx.commit().await?;
            self.store.forget_legacy_hidden(&user_id).await?;
        }
        Ok(imported)
    }
}
