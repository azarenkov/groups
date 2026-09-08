use std::sync::Arc;

use domain::user::id::UserId;
use identity_domain::unit_of_work::IdentityUnitOfWork;
use identity_domain::user::repository::UserRepository;
use identity_domain::user::settings::hotkeys::{HotkeyBindings, HotkeyError, HotkeyPatch};
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum HotkeySettingsError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Invalid hotkey settings: {0}")]
    Invalid(#[from] HotkeyError),
}

pub struct GetHotkeySettings<U: UserRepository> {
    user_repository: Arc<U>,
}

impl<U: UserRepository> GetHotkeySettings<U> {
    pub fn new(user_repository: Arc<U>) -> Self {
        Self { user_repository }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
    ) -> Result<Option<HotkeyBindings>, HotkeySettingsError> {
        Ok(self.user_repository.get_hotkey_settings(&user_id).await?)
    }
}

pub struct UpdateHotkeySettings<W: IdentityUnitOfWork> {
    unit_of_work: Arc<W>,
}

impl<W: IdentityUnitOfWork> UpdateHotkeySettings<W> {
    pub fn new(unit_of_work: Arc<W>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        patch: HotkeyPatch,
    ) -> Result<HotkeyBindings, HotkeySettingsError> {
        let mut tx = self.unit_of_work.begin().await?;
        let current = tx
            .load_hotkey_settings_for_update(&user_id)
            .await?
            .unwrap_or_default();
        let next = current.apply(&patch)?;
        tx.save_hotkey_settings(&user_id, &next).await?;
        tx.commit().await?;
        Ok(next)
    }
}
