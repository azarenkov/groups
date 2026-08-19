use async_trait::async_trait;

use crate::{
    server::id::ServerId, shared::repository::error::RepositoryError, user::id::UserId,
};

#[async_trait]
pub trait MediaRepository {
    async fn get_user_avatar_key(
        &self,
        user_id: &UserId,
    ) -> Result<Option<String>, RepositoryError>;
    async fn set_user_avatar_key(
        &self,
        user_id: &UserId,
        key: Option<&str>,
    ) -> Result<Option<String>, RepositoryError>;
    async fn get_server_logo_key(
        &self,
        server_id: &ServerId,
    ) -> Result<Option<String>, RepositoryError>;
    async fn set_server_logo_key(
        &self,
        server_id: &ServerId,
        key: Option<&str>,
    ) -> Result<Option<String>, RepositoryError>;
    async fn get_server_owner(
        &self,
        server_id: &ServerId,
    ) -> Result<Option<UserId>, RepositoryError>;
    async fn upsert_server_owner(
        &self,
        server_id: &ServerId,
        owner_id: &UserId,
    ) -> Result<(), RepositoryError>;
    async fn remove_server_media(&self, server_id: &ServerId) -> Result<(), RepositoryError>;
}
