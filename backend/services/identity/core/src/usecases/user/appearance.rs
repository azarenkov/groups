use std::sync::Arc;

use domain::user::id::UserId;
use identity_domain::user::repository::UserRepository;
use ports::repository::RepositoryError;

#[derive(Debug, Clone)]
pub struct AppearanceSettings {
    pub theme: String,
    pub accent: String,
    pub chat_font_size: i16,
    pub reduce_motion: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum AppearanceError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Invalid appearance settings")]
    Invalid,
}

pub struct AppearancePatch {
    pub theme: Option<String>,
    pub accent: Option<String>,
    pub chat_font_size: Option<i16>,
    pub reduce_motion: Option<bool>,
}

fn valid_theme(theme: &str) -> bool {
    matches!(theme, "light" | "dark" | "system")
}

fn valid_accent(accent: &str) -> bool {
    accent.is_empty()
        || (accent.len() == 7
            && accent.starts_with('#')
            && accent[1..].chars().all(|c| c.is_ascii_hexdigit()))
}

fn from_row(row: (String, String, i16, bool)) -> AppearanceSettings {
    AppearanceSettings {
        theme: row.0,
        accent: row.1,
        chat_font_size: row.2,
        reduce_motion: row.3,
    }
}

pub struct GetAppearance<U: UserRepository> {
    user_repository: Arc<U>,
}

impl<U: UserRepository> GetAppearance<U> {
    pub fn new(user_repository: Arc<U>) -> Self {
        Self { user_repository }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
    ) -> Result<Option<AppearanceSettings>, AppearanceError> {
        Ok(self
            .user_repository
            .get_appearance(&user_id)
            .await?
            .map(from_row))
    }
}

pub struct UpdateAppearance<U: UserRepository> {
    user_repository: Arc<U>,
}

impl<U: UserRepository> UpdateAppearance<U> {
    pub fn new(user_repository: Arc<U>) -> Self {
        Self { user_repository }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        patch: AppearancePatch,
    ) -> Result<AppearanceSettings, AppearanceError> {
        if patch.theme.as_deref().is_some_and(|t| !valid_theme(t)) {
            return Err(AppearanceError::Invalid);
        }
        if patch.accent.as_deref().is_some_and(|a| !valid_accent(a)) {
            return Err(AppearanceError::Invalid);
        }
        if patch.chat_font_size.is_some_and(|s| !(12..=18).contains(&s)) {
            return Err(AppearanceError::Invalid);
        }
        Ok(from_row(
            self.user_repository
                .apply_appearance_patch(
                    &user_id,
                    patch.theme.as_deref(),
                    patch.accent.as_deref(),
                    patch.chat_font_size,
                    patch.reduce_motion,
                )
                .await?,
        ))
    }
}
