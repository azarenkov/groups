use async_trait::async_trait;

use crate::{
    room::Room,
    server::{Server, value_objects::id::ServerId},
    shared::repository::error::RepositoryError,
    user::{User, value_objects::id::UserId},
};

#[async_trait]
pub trait ServerRepository {
    async fn server_exists(&self, server_id: &ServerId) -> Result<bool, RepositoryError>;
    async fn create_server(&self, server: &Server) -> Result<(), RepositoryError>;
    async fn get_servers_by_user_id(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<Server>, RepositoryError>;
    async fn get_users_by_server_id(
        &self,
        server_id: &ServerId,
    ) -> Result<Vec<User>, RepositoryError>;
    async fn add_member_to_server(
        &self,
        server_id: &ServerId,
        user_id: &UserId,
    ) -> Result<(), RepositoryError>;
    async fn get_rooms_by_server_id(
        &self,
        server_id: &ServerId,
    ) -> Result<Vec<Room>, RepositoryError>;
    async fn delete_server_by_owner(
        &self,
        server_id: &ServerId,
        owner_id: &UserId,
    ) -> Result<bool, RepositoryError>;
}
