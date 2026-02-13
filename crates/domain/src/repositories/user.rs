use async_trait::async_trait;

use crate::{
    entites::user::User,
    repositories::error::RepositoryError,
    value_objects::{
        password_hash::PasswordHash,
        user::{email::Email, id::UserId, name::UserName},
    },
};

#[async_trait]
pub trait UserRepository {
    async fn create_user(&self, user: &User) -> Result<(), RepositoryError>;
    async fn get_password_hash_by_email(
        &self,
        email: &Email,
    ) -> Result<PasswordHash, RepositoryError>;
    async fn get_username_by_id(&self, user_id: &UserId) -> Result<UserName, RepositoryError>;
    async fn get_user_id_by_email(&self, email: &Email) -> Result<UserId, RepositoryError>;
}
