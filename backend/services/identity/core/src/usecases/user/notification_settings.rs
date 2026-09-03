use std::sync::Arc;

use domain::user::id::UserId;
use identity_domain::user::repository::UserRepository;
use ports::repository::RepositoryError;

#[derive(Debug, Clone)]
pub struct NotificationSettings {
    pub desktop_enabled: bool,
    pub sounds_enabled: bool,
    pub mentions: String,
    pub quiet_hours_enabled: bool,
    pub quiet_start: String,
    pub quiet_end: String,
    pub calls_enabled: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum NotificationSettingsError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Invalid notification settings")]
    Invalid,
}

pub struct NotificationSettingsPatch {
    pub desktop_enabled: Option<bool>,
    pub sounds_enabled: Option<bool>,
    pub mentions: Option<String>,
    pub quiet_hours_enabled: Option<bool>,
    pub quiet_start: Option<String>,
    pub quiet_end: Option<String>,
    pub calls_enabled: Option<bool>,
}

fn from_row(row: (bool, bool, String, bool, String, String, bool)) -> NotificationSettings {
    NotificationSettings {
        desktop_enabled: row.0,
        sounds_enabled: row.1,
        mentions: row.2,
        quiet_hours_enabled: row.3,
        quiet_start: row.4,
        quiet_end: row.5,
        calls_enabled: row.6,
    }
}

fn valid_mentions(value: &str) -> bool {
    matches!(value, "all" | "mentions" | "none")
}

fn valid_time(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 5 || bytes[2] != b':' {
        return false;
    }
    let digits = [bytes[0], bytes[1], bytes[3], bytes[4]];
    if digits.iter().any(|b| !b.is_ascii_digit()) {
        return false;
    }
    let hours = (bytes[0] - b'0') * 10 + (bytes[1] - b'0');
    let minutes = (bytes[3] - b'0') * 10 + (bytes[4] - b'0');
    hours < 24 && minutes < 60
}

pub struct GetNotificationSettings<U: UserRepository> {
    user_repository: Arc<U>,
}

impl<U: UserRepository> GetNotificationSettings<U> {
    pub fn new(user_repository: Arc<U>) -> Self {
        Self { user_repository }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
    ) -> Result<Option<NotificationSettings>, NotificationSettingsError> {
        Ok(self
            .user_repository
            .get_notification_settings(&user_id)
            .await?
            .map(from_row))
    }
}

pub struct UpdateNotificationSettings<U: UserRepository> {
    user_repository: Arc<U>,
}

impl<U: UserRepository> UpdateNotificationSettings<U> {
    pub fn new(user_repository: Arc<U>) -> Self {
        Self { user_repository }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        patch: NotificationSettingsPatch,
    ) -> Result<NotificationSettings, NotificationSettingsError> {
        if patch.mentions.as_deref().is_some_and(|m| !valid_mentions(m))
            || patch.quiet_start.as_deref().is_some_and(|t| !valid_time(t))
            || patch.quiet_end.as_deref().is_some_and(|t| !valid_time(t))
        {
            return Err(NotificationSettingsError::Invalid);
        }
        Ok(from_row(
            self.user_repository
                .apply_notification_settings_patch(
                    &user_id,
                    patch.desktop_enabled,
                    patch.sounds_enabled,
                    patch.mentions.as_deref(),
                    patch.quiet_hours_enabled,
                    patch.quiet_start.as_deref(),
                    patch.quiet_end.as_deref(),
                    patch.calls_enabled,
                )
                .await?,
        ))
    }
}
