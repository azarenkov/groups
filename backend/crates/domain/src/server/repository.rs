use async_trait::async_trait;

use crate::{
    channel::Channel,
    server::{Server, id::ServerId},
    shared::{
        pagination::{PaginatedResult, Pagination},
        repository::error::RepositoryError,
    },
    user::{User, id::UserId},
};

#[async_trait]
pub trait ServerRepository {
    async fn server_exists(&self, server_id: &ServerId) -> Result<bool, RepositoryError>;
    async fn save_server(&self, server: &Server) -> Result<(), RepositoryError>;
    async fn find_servers_by_user_id(
        &self,
        user_id: &UserId,
        pagination: Pagination,
    ) -> Result<PaginatedResult<Server>, RepositoryError>;
    async fn find_users_by_server_id(
        &self,
        server_id: &ServerId,
        pagination: Pagination,
    ) -> Result<PaginatedResult<User>, RepositoryError>;
    async fn find_channels_by_server_id(
        &self,
        server_id: &ServerId,
        pagination: Pagination,
    ) -> Result<PaginatedResult<Channel>, RepositoryError>;
    async fn save_new_member_server(
        &self,
        server_id: &ServerId,
        user_id: &UserId,
    ) -> Result<(), RepositoryError>;
    async fn delete_server_by_owner(
        &self,
        server_id: &ServerId,
        owner_id: &UserId,
    ) -> Result<bool, RepositoryError>;
}
