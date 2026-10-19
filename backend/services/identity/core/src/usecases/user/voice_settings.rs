use std::sync::Arc;

use domain::user::id::UserId;
use identity_domain::unit_of_work::IdentityUnitOfWork;
use identity_domain::user::repository::UserRepository;
use identity_domain::user::settings::voice::{
    VoiceSettings, VoiceSettingsError as DomainVoiceSettingsError, VoiceSettingsPatch,
};
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum VoiceSettingsError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Invalid voice settings: {0}")]
    Invalid(#[from] DomainVoiceSettingsError),
}

pub struct GetVoiceSettings<U: UserRepository> {
    user_repository: Arc<U>,
}

impl<U: UserRepository> GetVoiceSettings<U> {
    pub fn new(user_repository: Arc<U>) -> Self {
        Self { user_repository }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
    ) -> Result<Option<VoiceSettings>, VoiceSettingsError> {
        Ok(self.user_repository.get_voice_settings(&user_id).await?)
    }
}

pub struct UpdateVoiceSettings<W: IdentityUnitOfWork> {
    unit_of_work: Arc<W>,
}

impl<W: IdentityUnitOfWork> UpdateVoiceSettings<W> {
    pub fn new(unit_of_work: Arc<W>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        patch: VoiceSettingsPatch,
    ) -> Result<VoiceSettings, VoiceSettingsError> {
        let mut tx = self.unit_of_work.begin().await?;
        if !tx.lock_user(&user_id).await? {
            return Err(VoiceSettingsError::Repository(RepositoryError::NotFound));
        }
        let next = tx
            .load_voice_settings_for_update(&user_id)
            .await?
            .unwrap_or_default()
            .apply(&patch)?;
        tx.save_voice_settings(&user_id, &next).await?;
        tx.commit().await?;
        Ok(next)
    }
}
