use async_trait::async_trait;

use crate::{
    channel::Channel,
    dm_message::id::DmMessageId,
    server::{Server, id::ServerId, invite_code::InviteCode},
    shared::{
        outbox::outbox_record::OutboxRecord,
        pagination::{PaginatedResult, Pagination},
        repository::error::RepositoryError,
    },
    user::{User, id::UserId},
};

#[async_trait]
pub trait ServerRepository {
    async fn server_exists(&self, server_id: &ServerId) -> Result<bool, RepositoryError>;
    async fn save_server_with_events(
        &self,
        server: &Server,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError>;
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
    async fn find_member_ids_by_server_id(
        &self,
        server_id: &ServerId,
    ) -> Result<Vec<UserId>, RepositoryError>;
    async fn find_channels_by_server_id(
        &self,
        server_id: &ServerId,
        pagination: Pagination,
    ) -> Result<PaginatedResult<Channel>, RepositoryError>;
    async fn accept_invite(
        &self,
        invite_message_id: &DmMessageId,
        server_id: &ServerId,
        user_id: &UserId,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError>;
    async fn remove_server_member(
        &self,
        server_id: &ServerId,
        user_id: &UserId,
    ) -> Result<bool, RepositoryError>;
    async fn delete_server_by_owner_with_events(
        &self,
        server_id: &ServerId,
        owner_id: &UserId,
        events: &[OutboxRecord],
    ) -> Result<Option<Vec<String>>, RepositoryError>;
    async fn find_server_owner(
        &self,
        server_id: &ServerId,
    ) -> Result<Option<UserId>, RepositoryError>;
    async fn find_server_by_id(
        &self,
        server_id: &ServerId,
    ) -> Result<Option<Server>, RepositoryError>;
    async fn is_server_member(
        &self,
        server_id: &ServerId,
        user_id: &UserId,
    ) -> Result<bool, RepositoryError>;
    async fn update_server_details(
        &self,
        server_id: &ServerId,
        name: &str,
        description: Option<&str>,
        events: &[OutboxRecord],
    ) -> Result<bool, RepositoryError>;
    async fn create_invite_code(
        &self,
        server_id: &ServerId,
        created_by: &UserId,
        code: &str,
        max_uses: Option<i32>,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<InviteCode, RepositoryError>;
    async fn list_invite_codes(
        &self,
        server_id: &ServerId,
    ) -> Result<Vec<InviteCode>, RepositoryError>;
    async fn revoke_invite_code(
        &self,
        server_id: &ServerId,
        code: &str,
        requester: &UserId,
        is_owner: bool,
    ) -> Result<bool, RepositoryError>;
    async fn redeem_invite_code(
        &self,
        code: &str,
        user_id: &UserId,
    ) -> Result<Option<ServerId>, RepositoryError>;
    async fn get_logo_key(&self, server_id: &ServerId)
    -> Result<Option<String>, RepositoryError>;
    async fn set_logo_key(
        &self,
        server_id: &ServerId,
        key: Option<&str>,
    ) -> Result<Option<String>, RepositoryError>;
}
