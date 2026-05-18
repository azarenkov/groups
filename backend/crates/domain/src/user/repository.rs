use async_trait::async_trait;

use crate::{
    refresh_token::RefreshToken,
    shared::{
        outbox::outbox_record::OutboxRecord, repository::error::RepositoryError,
        value_objects::password_hash::PasswordHash,
    },
    user::{User, email::Email, id::UserId, name::UserName},
    verification::code::VerificationCode,
};

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
}
