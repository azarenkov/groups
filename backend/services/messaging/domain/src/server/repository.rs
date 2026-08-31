use async_trait::async_trait;

use crate::{channel::Channel, server::Server, server::id::ServerId, server::invite_code::InviteCode};
use domain::{pagination::PaginatedResult, pagination::Pagination, user::id::UserId, user::name::UserName};
use ports::repository::RepositoryError;

#[derive(Debug, Clone)]
pub struct MutualServer {
    pub server_id: ServerId,
    pub name: String,
    pub accent_color: Option<String>,
}

#[async_trait]
pub trait ServerRepository {
    async fn server_exists(&self, server_id: &ServerId) -> Result<bool, RepositoryError>;
    async fn find_servers_by_user_id(
        &self,
        user_id: &UserId,
        pagination: Pagination,
    ) -> Result<PaginatedResult<Server>, RepositoryError>;
    async fn find_users_by_server_id(
        &self,
        server_id: &ServerId,
        pagination: Pagination,
    ) -> Result<PaginatedResult<(UserId, UserName)>, RepositoryError>;
    async fn find_member_ids_by_server_id(
        &self,
        server_id: &ServerId,
    ) -> Result<Vec<UserId>, RepositoryError>;
    async fn find_channels_by_server_id(
        &self,
        server_id: &ServerId,
        pagination: Pagination,
    ) -> Result<PaginatedResult<Channel>, RepositoryError>;
    async fn find_server_owner(
        &self,
        server_id: &ServerId,
    ) -> Result<Option<UserId>, RepositoryError>;
    async fn find_mutual_servers(
        &self,
        user_a: &UserId,
        user_b: &UserId,
    ) -> Result<Vec<MutualServer>, RepositoryError>;

    async fn find_server_by_id(
        &self,
        server_id: &ServerId,
    ) -> Result<Option<Server>, RepositoryError>;
    async fn is_server_member(
        &self,
        server_id: &ServerId,
        user_id: &UserId,
    ) -> Result<bool, RepositoryError>;
    async fn list_invite_codes(
        &self,
        server_id: &ServerId,
    ) -> Result<Vec<InviteCode>, RepositoryError>;
    async fn redeem_invite_code(
        &self,
        code: &str,
        user_id: &UserId,
    ) -> Result<Option<ServerId>, RepositoryError>;

}
