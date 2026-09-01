use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::{user::User, user::email::Email, user::id::UserId, user::name::UserName};
use domain::password_hash::PasswordHash;
use ports::repository::RepositoryError;

#[async_trait]
pub trait UserRepository {
    async fn save_user(&self, user: &User) -> Result<(), RepositoryError>;
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
    async fn list_oauth_connections(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<(String, Option<String>)>, RepositoryError>;
    async fn link_connection(
        &self,
        provider: &str,
        subject: &str,
        user_id: &UserId,
        display_name: &str,
    ) -> Result<bool, RepositoryError>;
    async fn unlink_connection(
        &self,
        user_id: &UserId,
        provider: &str,
    ) -> Result<bool, RepositoryError>;
    async fn create_connection_state(
        &self,
        state: uuid::Uuid,
        user_id: &UserId,
        provider: &str,
    ) -> Result<(), RepositoryError>;
    async fn consume_connection_state(
        &self,
        state: uuid::Uuid,
    ) -> Result<Option<(UserId, String, DateTime<Utc>)>, RepositoryError>;

}
