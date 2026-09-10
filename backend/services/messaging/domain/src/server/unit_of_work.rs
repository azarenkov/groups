use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{events::MessagingEvent, channel::Channel, channel::id::ChannelId, channel::participant_deafen::ParticipantDeafen, channel::participant_mute::ParticipantMute, dm_message::DmMessage, dm_message::id::DmMessageId, message::id::MessageId, server::Server, server::id::ServerId, server::invite_link::InviteLink, server::invite_link::InviteToken, server::role::ServerRole};
use domain::user::id::UserId;
use ports::repository::RepositoryError;

#[async_trait::async_trait]
pub trait ServerUnitOfWork: Send + Sync {
    async fn begin(&self, server_id: &ServerId) -> Result<Box<dyn ServerTransaction>, RepositoryError>;
    async fn begin_user(&self, user_id: &UserId) -> Result<Box<dyn UserLifecycleTransaction>, RepositoryError>;
}

#[async_trait::async_trait]
pub trait UserLifecycleTransaction: Send {
    async fn mark_user_deleted(&mut self, deleted_at: DateTime<Utc>) -> Result<(), RepositoryError>;
    async fn owned_server_ids(&mut self) -> Result<Vec<ServerId>, RepositoryError>;
    async fn commit(self: Box<Self>) -> Result<(), RepositoryError>;
}

#[async_trait::async_trait]
pub trait ServerTransaction: Send {
    async fn load_server(&mut self) -> Result<Server, RepositoryError>;
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
    async fn load_channel(&mut self, channel_id: &ChannelId) -> Result<Channel, RepositoryError>;
    async fn save_mute_state(
        &mut self,
        channel_id: &ChannelId,
        user_id: &UserId,
        mute: Option<&ParticipantMute>,
    ) -> Result<(), RepositoryError>;
    async fn save_deafen_state(
        &mut self,
        channel_id: &ChannelId,
        user_id: &UserId,
        deafen: Option<&ParticipantDeafen>,
    ) -> Result<(), RepositoryError>;
    async fn insert_channel(&mut self, channel: &Channel) -> Result<(), RepositoryError>;
    async fn delete_channel(&mut self, channel_id: &ChannelId) -> Result<Vec<String>, RepositoryError>;
    async fn delete_message(&mut self, message_id: &MessageId) -> Result<(), RepositoryError>;
    async fn update_server_details(
        &mut self,
        name: &str,
        description: Option<&str>,
    ) -> Result<bool, RepositoryError>;
    async fn update_server_owner(&mut self, owner_id: &UserId) -> Result<bool, RepositoryError>;
    async fn find_longest_member_excluding(&mut self, exclude: &UserId) -> Result<Option<UserId>, RepositoryError>;
    async fn create_invite_link(&mut self, link: &InviteLink) -> Result<(), RepositoryError>;
    async fn find_primary_invite_link(&mut self) -> Result<Option<InviteLink>, RepositoryError>;
    async fn revoke_invite_link(
        &mut self,
        token: &InviteToken,
        requester: &UserId,
        any_creator: bool,
    ) -> Result<Option<InviteLink>, RepositoryError>;
    async fn insert_dm_message(&mut self, message: &DmMessage) -> Result<(), RepositoryError>;
    async fn insert_server(&mut self, server: &Server) -> Result<(), RepositoryError>;
    async fn is_user_deleted(&mut self, user_id: &UserId) -> Result<bool, RepositoryError>;
    async fn lock_user(&mut self, user_id: &UserId) -> Result<(), RepositoryError>;
    async fn accept_invite(
        &mut self,
        invite_message_id: &DmMessageId,
        user_id: &UserId,
    ) -> Result<(), RepositoryError>;
    async fn delete_server_by_owner(&mut self, owner_id: &UserId) -> Result<Option<Vec<String>>, RepositoryError>;
    async fn save_events(&mut self, events: &[MessagingEvent], request_id: Option<&str>) -> Result<(), RepositoryError>;
    async fn commit(self: Box<Self>) -> Result<(), RepositoryError>;
}
