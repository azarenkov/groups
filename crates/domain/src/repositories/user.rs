use async_trait::async_trait;

use crate::{
    repositories::error::RepositoryError,
    value_objects::user::{id::UserId, name::UserName},
};

#[async_trait]
pub trait UserRepository {
    async fn get_username_by_id(&self, user_id: &UserId) -> Result<UserName, RepositoryError>;
}
