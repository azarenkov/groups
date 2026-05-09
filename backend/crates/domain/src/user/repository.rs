use async_trait::async_trait;

use crate::{
    shared::{repository::error::RepositoryError, value_objects::password_hash::PasswordHash},
    user::{
        User,
        value_objects::{email::Email, id::UserId, name::UserName},
    },
};

#[async_trait]
pub trait UserRepository {
    async fn save_user(&self, user: &User) -> Result<(), RepositoryError>;
    async fn get_password_hash_by_email(
        &self,
        email: &Email,
    ) -> Result<PasswordHash, RepositoryError>;
    async fn get_username_by_id(&self, user_id: &UserId) -> Result<UserName, RepositoryError>;
    async fn get_user_id_by_email(&self, email: &Email) -> Result<UserId, RepositoryError>;
}
