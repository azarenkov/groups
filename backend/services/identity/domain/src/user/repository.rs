use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::{refresh_token::RefreshToken, user::User, user::email::Email, user::id::UserId, user::locale::Locale, user::name::UserName, verification::code::VerificationCode};
use domain::{shared::outbox::outbox_record::OutboxRecord, shared::repository::error::RepositoryError, shared::value_objects::password_hash::PasswordHash};

#[async_trait]
pub trait UserRepository {
    async fn save_user(&self, user: &User) -> Result<(), RepositoryError>;
    async fn register_user(
        &self,
        user: &User,
        refresh_token: &RefreshToken,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError>;
    async fn find_by_id(&self, user_id: &UserId) -> Result<Option<User>, RepositoryError>;
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepositoryError>;
    async fn find_by_name(&self, name: &UserName) -> Result<Option<User>, RepositoryError>;
    async fn ids_by_names(
        &self,
        lowercase_names: &[String],
    ) -> Result<Vec<(UserId, UserName)>, RepositoryError>;

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
    async fn confirm_registration(
        &self,
        user: &User,
        verification_code: &VerificationCode,
        events: &[OutboxRecord],
    ) -> Result<bool, RepositoryError>;
    async fn update_locale(
        &self,
        user_id: &UserId,
        locale: Locale,
    ) -> Result<(), RepositoryError>;
    async fn set_name_with_events(
        &self,
        user_id: &UserId,
        name: &UserName,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError>;
    async fn claim_with_oauth(
        &self,
        user_id: &UserId,
        password_hash: &PasswordHash,
        verified_at: DateTime<Utc>,
        provider: &str,
        subject: &str,
        email: &Email,
    ) -> Result<(), RepositoryError>;
    async fn find_user_id_by_oauth(
        &self,
        provider: &str,
        subject: &str,
    ) -> Result<Option<UserId>, RepositoryError>;
    async fn find_oauth_subject_for_user(
        &self,
        provider: &str,
        user_id: &UserId,
    ) -> Result<Option<String>, RepositoryError>;
    async fn link_oauth_identity(
        &self,
        provider: &str,
        subject: &str,
        user_id: &UserId,
        email: &Email,
    ) -> Result<(), RepositoryError>;

}
