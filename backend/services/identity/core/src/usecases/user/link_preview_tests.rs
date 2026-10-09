use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::{password_hash::PasswordHash, user::email::Email, user::locale::Locale, user::name::UserName};
use identity_domain::user::{
    User,
    settings::{appearance::AppearanceSettings, hotkeys::HotkeyBindings, notification::NotificationSettings, voice::VoiceSettings},
};

use super::*;

struct Users(Mutex<Vec<User>>);

#[async_trait]
impl UserRepository for Users {
    async fn find_by_id(&self, user_id: &UserId) -> Result<Option<User>, RepositoryError> {
        Ok(self.0.lock().unwrap().iter().find(|u| u.id() == user_id).map(|u| {
            User::restore(
                *u.id(),
                u.name().clone(),
                u.email().clone(),
                PasswordHash::new(u.password_hash().as_str().to_string()),
                u.created_at(),
                u.updated_at(),
                u.email_verified_at(),
                u.phone_verified_at(),
                u.locale(),
                u.pending_email().cloned(),
                u.password_changed_at(),
                u.disabled_at(),
                u.deleted_at(),
                u.bio().to_string(),
                u.badge_style(),
                u.show_activity(),
                u.show_game_activity(),
                u.timezone().to_string(),
            )
        }))
    }
    async fn save_user(&self, _: &User) -> Result<(), RepositoryError> { unimplemented!() }
    async fn find_by_email(&self, _: &Email) -> Result<Option<User>, RepositoryError> { unimplemented!() }
    async fn find_by_email_ignoring_case(&self, _: &Email) -> Result<Option<User>, RepositoryError> { unimplemented!() }
    async fn find_by_name(&self, _: &UserName) -> Result<Option<User>, RepositoryError> { unimplemented!() }
    async fn ids_by_names(&self, _: &[String]) -> Result<Vec<(UserId, UserName)>, RepositoryError> { unimplemented!() }
    async fn search_by_name_prefix(&self, _: &str, _: i64, _: &UserId) -> Result<Vec<(UserId, UserName)>, RepositoryError> { unimplemented!() }
    async fn get_password_hash_by_email(&self, _: &Email) -> Result<PasswordHash, RepositoryError> { unimplemented!() }
    async fn get_username_by_id(&self, _: &UserId) -> Result<UserName, RepositoryError> { unimplemented!() }
    async fn get_user_id_by_email(&self, _: &Email) -> Result<UserId, RepositoryError> { unimplemented!() }
    async fn list_oauth_connections(&self, _: &UserId) -> Result<Vec<(String, String, Option<String>)>, RepositoryError> { unimplemented!() }
    async fn link_connection(&self, _: &str, _: &str, _: &UserId, _: &str) -> Result<bool, RepositoryError> { unimplemented!() }
    async fn unlink_connection(&self, _: &UserId, _: &str) -> Result<bool, RepositoryError> { unimplemented!() }
    async fn create_connection_state(&self, _: uuid::Uuid, _: &UserId, _: &str) -> Result<(), RepositoryError> { unimplemented!() }
    async fn consume_connection_state(&self, _: uuid::Uuid) -> Result<Option<(UserId, String, DateTime<Utc>)>, RepositoryError> { unimplemented!() }
    async fn get_appearance(&self, _: &UserId) -> Result<Option<AppearanceSettings>, RepositoryError> { unimplemented!() }
    async fn get_voice_settings(&self, _: &UserId) -> Result<Option<VoiceSettings>, RepositoryError> { unimplemented!() }
    async fn get_notification_settings(&self, _: &UserId) -> Result<Option<NotificationSettings>, RepositoryError> { unimplemented!() }
    async fn get_hotkey_settings(&self, _: &UserId) -> Result<Option<HotkeyBindings>, RepositoryError> { unimplemented!() }
}

fn user(name: &str, verified: bool) -> User {
    let mut user = User::create(
        UserName::new(name.to_string()),
        Email::new(format!("{name}@example.com")),
        PasswordHash::new("h".to_string()),
        None,
        None,
        Locale::default(),
    );
    if verified {
        user.verify_email(Utc::now());
    }
    user.pull_events();
    user
}

fn usecase(users: Vec<User>) -> GetLinkPreview<Users> {
    GetLinkPreview::new(Arc::new(Users(Mutex::new(users))))
}

#[tokio::test]
async fn an_active_user_shows_their_name() {
    let alice = user("alice", true);
    let id = *alice.id();
    assert_eq!(usecase(vec![alice]).execute(id).await.unwrap().as_deref(), Some("alice"));
}

#[tokio::test]
async fn unknown_disabled_deleted_and_unverified_users_show_nothing() {
    let mut disabled = user("disabled", true);
    disabled.disable(Utc::now()).unwrap();
    let mut deleted = user("deleted", true);
    deleted.delete(Utc::now()).unwrap();
    let unverified = user("unverified", false);
    let ids = [*disabled.id(), *deleted.id(), *unverified.id(), UserId::generate()];
    let previews = usecase(vec![disabled, deleted, unverified]);
    for id in ids {
        assert_eq!(previews.execute(id).await.unwrap(), None);
    }
}
