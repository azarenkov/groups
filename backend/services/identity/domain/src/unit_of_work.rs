use domain::user::id::UserId;
use ports::repository::RepositoryError;

use chrono::{DateTime, Utc};

use crate::{events::IdentityEvent, user::email::Email, refresh_token::RefreshToken, refresh_token::id::RefreshTokenId, user::User, verification::code::VerificationCode, verification::purpose::VerificationPurpose};
use crate::user::settings::{
    appearance::AppearanceSettings, hotkeys::HotkeyBindings,
    notification::NotificationSettings, voice::VoiceSettings,
};

#[async_trait::async_trait]
pub trait IdentityUnitOfWork: Send + Sync {
    async fn begin(&self) -> Result<Box<dyn IdentityTransaction>, RepositoryError>;
}

#[async_trait::async_trait]
pub trait IdentityTransaction: Send {
    async fn load_user_for_update(&mut self, user_id: &UserId) -> Result<Option<User>, RepositoryError>;
    async fn load_stale_unverified_users_for_update(&mut self, registered_before: DateTime<Utc>, limit: i64) -> Result<Vec<User>, RepositoryError>;
    async fn load_pending_verification_for_update(&mut self, user_id: &UserId) -> Result<Option<VerificationCode>, RepositoryError>;
    async fn load_pending_verification_for_purpose_for_update(&mut self, user_id: &UserId, purpose: VerificationPurpose) -> Result<Option<VerificationCode>, RepositoryError>;
    async fn delete_oauth_identities(&mut self, user_id: &UserId) -> Result<(), RepositoryError>;
    async fn link_oauth_identity(&mut self, provider: &str, subject: &str, user_id: &UserId, email: &Email) -> Result<bool, RepositoryError>;
    async fn revoke_refresh_tokens_for_user(&mut self, user_id: &UserId, keep: Option<&RefreshTokenId>, now: DateTime<Utc>) -> Result<(), RepositoryError>;
    async fn load_refresh_token_for_update(&mut self, id: &RefreshTokenId) -> Result<Option<RefreshToken>, RepositoryError>;
    async fn insert_user(&mut self, user: &User) -> Result<(), RepositoryError>;
    async fn save_user(&mut self, user: &User) -> Result<(), RepositoryError>;
    async fn insert_refresh_token(&mut self, refresh_token: &RefreshToken) -> Result<(), RepositoryError>;
    async fn save_refresh_token(&mut self, refresh_token: &RefreshToken) -> Result<(), RepositoryError>;
    async fn insert_verification_code(&mut self, verification_code: &VerificationCode) -> Result<(), RepositoryError>;
    async fn save_verification_code(&mut self, verification_code: &VerificationCode) -> Result<(), RepositoryError>;
    async fn save_entitlements(&mut self, user_id: &UserId, tier: &str, valid_until: Option<DateTime<Utc>>, source: &str) -> Result<(), RepositoryError>;
    async fn save_events(&mut self, events: &[IdentityEvent], request_id: Option<&str>) -> Result<(), RepositoryError>;
    async fn load_appearance_for_update(&mut self, user_id: &UserId) -> Result<Option<AppearanceSettings>, RepositoryError>;
    async fn save_appearance(&mut self, user_id: &UserId, settings: &AppearanceSettings) -> Result<(), RepositoryError>;
    async fn load_voice_settings_for_update(&mut self, user_id: &UserId) -> Result<Option<VoiceSettings>, RepositoryError>;
    async fn save_voice_settings(&mut self, user_id: &UserId, settings: &VoiceSettings) -> Result<(), RepositoryError>;
    async fn load_notification_settings_for_update(&mut self, user_id: &UserId) -> Result<Option<NotificationSettings>, RepositoryError>;
    async fn save_notification_settings(&mut self, user_id: &UserId, settings: &NotificationSettings) -> Result<(), RepositoryError>;
    async fn load_hotkey_settings_for_update(&mut self, user_id: &UserId) -> Result<Option<HotkeyBindings>, RepositoryError>;
    async fn save_hotkey_settings(&mut self, user_id: &UserId, settings: &HotkeyBindings) -> Result<(), RepositoryError>;
    async fn commit(self: Box<Self>) -> Result<(), RepositoryError>;
}
