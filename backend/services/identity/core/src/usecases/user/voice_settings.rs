use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use identity_domain::unit_of_work::IdentityUnitOfWork;
use identity_domain::user::repository::UserRepository;
use identity_domain::user::settings::voice::{
    NoiseMode, VoiceSettings, VoiceSettingsError as DomainVoiceSettingsError, VoiceSettingsPatch,
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
        if self.entitlements.get(&user_id).await?.active_at(Utc::now()).is_paid() {
            return Ok(Some(settings));
        }
        Ok(Some(settings.without_paid_features()))
    }
}

pub struct UpdateVoiceSettings<W: IdentityUnitOfWork, E: EntitlementsRepository> {
    unit_of_work: Arc<W>,
    entitlements: Arc<E>,
}

impl<W: IdentityUnitOfWork, E: EntitlementsRepository> UpdateVoiceSettings<W, E> {
    pub fn new(unit_of_work: Arc<W>, entitlements: Arc<E>) -> Self {
        Self {
            unit_of_work,
            entitlements,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        patch: VoiceSettingsPatch,
    ) -> Result<VoiceSettings, VoiceSettingsError> {
        let paid = self.entitlements.get(&user_id).await?.active_at(Utc::now()).is_paid();
        if !paid && patch.noise_mode.as_deref() == Some(NoiseMode::High.as_str()) {
            return Err(VoiceSettingsError::ProRequired);
        }
        let mut tx = self.unit_of_work.begin().await?;
        let current = tx
            .load_voice_settings_for_update(&user_id)
            .await?
            .unwrap_or_default();
        let next = current.apply(&patch)?;
        let next = if paid { next } else { next.without_paid_features() };
        tx.save_voice_settings(&user_id, &next).await?;
        tx.commit().await?;
        Ok(next)
    }
}
