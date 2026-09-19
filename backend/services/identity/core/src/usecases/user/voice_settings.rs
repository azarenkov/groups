use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use identity_domain::unit_of_work::IdentityUnitOfWork;
use identity_domain::user::repository::UserRepository;
use identity_domain::user::settings::voice::{
    VoiceSettings, VoiceSettingsError as DomainVoiceSettingsError, VoiceSettingsPatch,
};
use identity_domain::user::tier::EntitlementsRepository;
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum VoiceSettingsError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Invalid voice settings: {0}")]
    Invalid(#[from] DomainVoiceSettingsError),
    #[error("A paid plan is required")]
    ProRequired,
}

pub struct GetVoiceSettings<U: UserRepository, E: EntitlementsRepository> {
    user_repository: Arc<U>,
    entitlements: Arc<E>,
}

impl<U: UserRepository, E: EntitlementsRepository> GetVoiceSettings<U, E> {
    pub fn new(user_repository: Arc<U>, entitlements: Arc<E>) -> Self {
        Self {
            user_repository,
            entitlements,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
    ) -> Result<Option<VoiceSettings>, VoiceSettingsError> {
        let Some(settings) = self.user_repository.get_voice_settings(&user_id).await? else {
            return Ok(None);
        };
        let tier = self.entitlements.get(&user_id).await?.active_at(Utc::now());
        Ok(Some(settings.limited_to(tier)))
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
        let tier = tx.load_entitlements(&user_id).await?.active_at(Utc::now());
        let current = tx
            .load_voice_settings_for_update(&user_id)
            .await?
            .unwrap_or_default();
        let requested = current.clone().limited_to(tier).apply(&patch)?;
        if !requested.allowed_for(tier) {
            return Err(VoiceSettingsError::ProRequired);
        }
        let next = requested;
        tx.save_voice_settings(&user_id, &next).await?;
        tx.commit().await?;
        Ok(next)
    }
}
