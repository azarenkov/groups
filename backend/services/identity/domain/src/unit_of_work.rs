use domain::user::id::UserId;
use ports::repository::RepositoryError;

use chrono::{DateTime, Utc};

use crate::{events::IdentityEvent, user::access_revocation::AccessRevocation, user::presence::PresencePreference, refresh_token::RefreshToken, refresh_token::id::RefreshTokenId, user::User, verification::code::VerificationCode, verification::id::VerificationCodeId, verification::purpose::VerificationPurpose};
use crate::verification::throttle::IssuedCodes;
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
    async fn load_pending_verification_for_purpose_for_update(&mut self, user_id: &UserId, purpose: VerificationPurpose) -> Result<Option<VerificationCode>, RepositoryError>;
    async fn revoke_refresh_tokens_for_user(&mut self, user_id: &UserId, keep: Option<&RefreshTokenId>, now: DateTime<Utc>) -> Result<(), RepositoryError>;
    async fn record_access_revocation(&mut self, revocation: &AccessRevocation) -> Result<(), RepositoryError>;
    async fn load_pending_access_revocations(&mut self, limit: i64) -> Result<Vec<AccessRevocation>, RepositoryError>;
    async fn clear_access_revocation(&mut self, revocation: &AccessRevocation) -> Result<(), RepositoryError>;
    async fn load_refresh_token_for_update(&mut self, id: &RefreshTokenId) -> Result<Option<RefreshToken>, RepositoryError>;
    async fn insert_user(&mut self, user: &User) -> Result<(), RepositoryError>;
    async fn save_user(&mut self, user: &User) -> Result<(), RepositoryError>;
    async fn insert_refresh_token(&mut self, refresh_token: &RefreshToken) -> Result<(), RepositoryError>;
    async fn save_refresh_token(&mut self, refresh_token: &RefreshToken) -> Result<(), RepositoryError>;
    async fn insert_verification_code(&mut self, verification_code: &VerificationCode) -> Result<(), RepositoryError>;
    async fn save_verification_code(&mut self, verification_code: &VerificationCode) -> Result<(), RepositoryError>;
    async fn verification_code_owner(&mut self, id: &VerificationCodeId) -> Result<Option<UserId>, RepositoryError>;
    async fn load_verification_code_for_update(&mut self, id: &VerificationCodeId) -> Result<Option<VerificationCode>, RepositoryError>;
    async fn consume_pending_verifications(&mut self, user_id: &UserId, purpose: VerificationPurpose, now: DateTime<Utc>) -> Result<(), RepositoryError>;
    async fn lock_user(&mut self, user_id: &UserId) -> Result<bool, RepositoryError>;
    async fn save_events(&mut self, events: &[IdentityEvent], request_id: Option<&str>) -> Result<(), RepositoryError>;
    async fn issued_verification_codes(&mut self, user_id: &UserId, purpose: VerificationPurpose, since: DateTime<Utc>) -> Result<IssuedCodes, RepositoryError>;
    async fn load_appearance_for_update(&mut self, user_id: &UserId) -> Result<Option<AppearanceSettings>, RepositoryError>;
    async fn save_appearance(&mut self, user_id: &UserId, settings: &AppearanceSettings) -> Result<(), RepositoryError>;
    async fn load_voice_settings_for_update(&mut self, user_id: &UserId) -> Result<Option<VoiceSettings>, RepositoryError>;
    async fn save_voice_settings(&mut self, user_id: &UserId, settings: &VoiceSettings) -> Result<(), RepositoryError>;
    async fn load_notification_settings_for_update(&mut self, user_id: &UserId) -> Result<Option<NotificationSettings>, RepositoryError>;
    async fn save_notification_settings(&mut self, user_id: &UserId, settings: &NotificationSettings) -> Result<(), RepositoryError>;
    async fn load_hotkey_settings_for_update(&mut self, user_id: &UserId) -> Result<Option<HotkeyBindings>, RepositoryError>;
    async fn save_hotkey_settings(&mut self, user_id: &UserId, settings: &HotkeyBindings) -> Result<(), RepositoryError>;
    async fn load_presence_preference_for_update(&mut self, user_id: &UserId) -> Result<Option<PresencePreference>, RepositoryError>;
    async fn save_presence_preference(&mut self, preference: &PresencePreference) -> Result<(), RepositoryError>;
    async fn commit(self: Box<Self>) -> Result<(), RepositoryError>;
}
