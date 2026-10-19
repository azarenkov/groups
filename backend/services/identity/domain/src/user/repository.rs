use async_trait::async_trait;
use crate::user::settings::{
    appearance::AppearanceSettings, hotkeys::HotkeyBindings,
    notification::NotificationSettings, voice::VoiceSettings,
};
use crate::{user::User, user::email::Email, user::id::UserId, user::name::UserName};
use domain::password_hash::PasswordHash;
use ports::repository::RepositoryError;

#[async_trait]
pub trait UserRepository {
    async fn save_user(&self, user: &User) -> Result<(), RepositoryError>;
    async fn find_by_id(&self, user_id: &UserId) -> Result<Option<User>, RepositoryError>;
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepositoryError>;
    async fn find_by_email_ignoring_case(&self, email: &Email) -> Result<Option<User>, RepositoryError>;
    async fn find_by_name(&self, name: &UserName) -> Result<Option<User>, RepositoryError>;

    async fn search_by_name_prefix(
        &self,
        prefix: &str,
        limit: i64,
        exclude_user: &UserId,
    ) -> Result<Vec<(UserId, UserName)>, RepositoryError>;
    async fn get_password_hash_by_email(
        &self,
        email: &Email,
    ) -> Result<PasswordHash, RepositoryError>;
    async fn get_username_by_id(&self, user_id: &UserId) -> Result<UserName, RepositoryError>;
    async fn get_user_id_by_email(&self, email: &Email) -> Result<UserId, RepositoryError>;
    async fn get_appearance(
        &self,
        user_id: &UserId,
    ) -> Result<Option<AppearanceSettings>, RepositoryError>;
    async fn get_voice_settings(
        &self,
        user_id: &UserId,
    ) -> Result<Option<VoiceSettings>, RepositoryError>;
    async fn get_notification_settings(
        &self,
        user_id: &UserId,
    ) -> Result<Option<NotificationSettings>, RepositoryError>;
    async fn appear_offline_among(&self, user_ids: &[UserId]) -> Result<Vec<UserId>, RepositoryError>;
    async fn get_hotkey_settings(
        &self,
        user_id: &UserId,
    ) -> Result<Option<HotkeyBindings>, RepositoryError>;
}
