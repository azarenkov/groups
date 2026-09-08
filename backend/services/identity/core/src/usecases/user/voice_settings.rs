use std::sync::Arc;

use domain::user::id::UserId;
use identity_domain::user::repository::UserRepository;
use ports::repository::RepositoryError;

#[derive(Debug, Clone)]
pub struct VoiceSettings {
    pub mic_volume: i16,
    pub output_volume: i16,
    pub noise_mode: String,
    pub auto_sensitivity: bool,
    pub sensitivity: i16,
    pub push_to_talk: bool,
    pub ptt_key: String,
}

#[derive(Debug, thiserror::Error)]
pub enum VoiceSettingsError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Invalid voice settings")]
    Invalid,
}

pub struct VoiceSettingsPatch {
    pub mic_volume: Option<i16>,
    pub output_volume: Option<i16>,
    pub noise_mode: Option<String>,
    pub auto_sensitivity: Option<bool>,
    pub sensitivity: Option<i16>,
    pub push_to_talk: Option<bool>,
    pub ptt_key: Option<String>,
}

fn from_row(row: (i16, i16, String, bool, i16, bool, String)) -> VoiceSettings {
    VoiceSettings {
        mic_volume: row.0,
        output_volume: row.1,
        noise_mode: row.2,
        auto_sensitivity: row.3,
        sensitivity: row.4,
        push_to_talk: row.5,
        ptt_key: row.6,
    }
}

fn valid_percent(value: i16) -> bool {
    (0..=100).contains(&value)
}

fn valid_noise_mode(mode: &str) -> bool {
    matches!(mode, "off" | "standard" | "high")
}

const MODIFIER_CODES: &[&str] = &[
    "ShiftLeft",
    "ShiftRight",
    "ControlLeft",
    "ControlRight",
    "AltLeft",
    "AltRight",
    "MetaLeft",
    "MetaRight",
];

fn valid_ptt_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 32
        && key.chars().all(|c| c.is_ascii_alphanumeric())
        && !MODIFIER_CODES.contains(&key)
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
        Ok(self
            .user_repository
            .get_voice_settings(&user_id)
            .await?
            .map(from_row))
    }
}

pub struct UpdateVoiceSettings<U: UserRepository> {
    user_repository: Arc<U>,
}

impl<U: UserRepository> UpdateVoiceSettings<U> {
    pub fn new(user_repository: Arc<U>) -> Self {
        Self { user_repository }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        patch: VoiceSettingsPatch,
    ) -> Result<VoiceSettings, VoiceSettingsError> {
        if patch.mic_volume.is_some_and(|v| !valid_percent(v))
            || patch.output_volume.is_some_and(|v| !valid_percent(v))
            || patch.sensitivity.is_some_and(|v| !valid_percent(v))
            || patch.noise_mode.as_deref().is_some_and(|m| !valid_noise_mode(m))
            || patch.ptt_key.as_deref().is_some_and(|k| !valid_ptt_key(k))
        {
            return Err(VoiceSettingsError::Invalid);
        }
        Ok(from_row(
            self.user_repository
                .apply_voice_settings_patch(
                    &user_id,
                    patch.mic_volume,
                    patch.output_volume,
                    patch.noise_mode.as_deref(),
                    patch.auto_sensitivity,
                    patch.sensitivity,
                    patch.push_to_talk,
                    patch.ptt_key.as_deref(),
                )
                .await?,
        ))
    }
}
