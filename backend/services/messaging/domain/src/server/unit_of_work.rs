use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{channel::Channel, channel::id::ChannelId, dm_message::DmMessage, dm_message::id::DmMessageId, message::id::MessageId, server::Server, server::authorization::AuthorizationSnapshot, server::id::ServerId, server::invite_code::InviteCode, server::role::ServerRole};
use domain::{shared::outbox::outbox_record::OutboxRecord, shared::repository::error::RepositoryError, user::id::UserId};

#[async_trait::async_trait]
pub trait ServerUnitOfWork: Send + Sync {
    async fn begin(&self, server_id: &ServerId) -> Result<Box<dyn ServerTransaction>, RepositoryError>;
}

#[async_trait::async_trait]
pub trait ServerTransaction: Send {
    async fn authorization(&mut self) -> Result<AuthorizationSnapshot, RepositoryError>;
    async fn insert_role(&mut self, role: &ServerRole) -> Result<(), RepositoryError>;
    async fn update_role(
        &mut self,
        role_id: Uuid,
        name: Option<&str>,
        color: Option<&str>,
        permissions: i64,
    ) -> Result<bool, RepositoryError>;
    async fn delete_role(&mut self, role_id: Uuid) -> Result<bool, RepositoryError>;
    async fn assign_role(&mut self, user_id: &UserId, role_id: Uuid) -> Result<(), RepositoryError>;
    async fn revoke_role(&mut self, user_id: &UserId, role_id: Uuid) -> Result<bool, RepositoryError>;
    async fn swap_role_positions(&mut self, role_a: Uuid, role_b: Uuid) -> Result<bool, RepositoryError>;
    async fn remove_member(&mut self, user_id: &UserId) -> Result<bool, RepositoryError>;
    async fn mute_participant(
        &mut self,
        channel_id: &ChannelId,
        target: &UserId,
        muted_by: &UserId,
        muted_at: DateTime<Utc>,
        self_muted: bool,
    ) -> Result<bool, RepositoryError>;
    async fn unmute_participant(
        &mut self,
        channel_id: &ChannelId,
        target: &UserId,
        expected_muted_by: Option<&UserId>,
        downgrade_to_self_mute: bool,
    ) -> Result<bool, RepositoryError>;
    async fn insert_channel(&mut self, channel: &Channel) -> Result<(), RepositoryError>;
    async fn delete_channel(&mut self, channel_id: &ChannelId) -> Result<Vec<String>, RepositoryError>;
    async fn delete_message(&mut self, message_id: &MessageId) -> Result<(), RepositoryError>;
    async fn update_server_details(
        &mut self,
        name: &str,
        description: Option<&str>,
    ) -> Result<bool, RepositoryError>;
    async fn create_invite_code(
        &mut self,
        created_by: &UserId,
        code: &str,
        max_uses: Option<i32>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<InviteCode, RepositoryError>;
    async fn revoke_invite_code(
        &mut self,
        code: &str,
        requester: &UserId,
        any_creator: bool,
    ) -> Result<bool, RepositoryError>;
    async fn insert_dm_message(&mut self, message: &DmMessage) -> Result<(), RepositoryError>;
    async fn insert_server(&mut self, server: &Server) -> Result<(), RepositoryError>;
    async fn accept_invite(
        &mut self,
        invite_message_id: &DmMessageId,
        user_id: &UserId,
    ) -> Result<(), RepositoryError>;
    async fn delete_server_by_owner(&mut self, owner_id: &UserId) -> Result<Option<Vec<String>>, RepositoryError>;
    async fn save_outbox(&mut self, records: &[OutboxRecord]) -> Result<(), RepositoryError>;
    async fn commit(self: Box<Self>) -> Result<(), RepositoryError>;
}
