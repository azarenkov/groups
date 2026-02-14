use async_trait::async_trait;

use crate::{
    entites::server::Server,
    repositories::error::RepositoryError,
    value_objects::{server::id::ServerId, user::id::UserId},
};

#[async_trait]
pub trait ServerRepository {
    async fn server_exists(&self, server_id: &ServerId) -> Result<bool, RepositoryError>;
    async fn create_server(&self, server: &Server) -> Result<(), RepositoryError>;
    async fn delete_server_by_owner(
        &self,
        server_id: &ServerId,
        owner_id: &UserId,
    ) -> Result<bool, RepositoryError>;
}
