use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{events::MessagingEvent, friendship::block::BlockRelation, channel::Channel, channel::id::ChannelId, channel::participant_deafen::ParticipantDeafen, channel::participant_mute::ParticipantMute, dm_message::DmMessage, dm_message::id::DmMessageId, message::Message, message::id::MessageId, community::Community, community::id::CommunityId, community::invite_link::InviteJoinOutcome, community::invite_link::InviteLink, community::invite_link::InviteToken, community::role::CommunityRole};
use domain::user::id::UserId;
use ports::repository::RepositoryError;

#[async_trait::async_trait]
pub trait CommunityUnitOfWork: Send + Sync {
    async fn begin(&self, community_id: &CommunityId) -> Result<Box<dyn CommunityTransaction>, RepositoryError>;
    async fn begin_user(&self, user_id: &UserId) -> Result<Box<dyn UserLifecycleTransaction>, RepositoryError>;
}

#[async_trait::async_trait]
pub trait UserLifecycleTransaction: Send {
    async fn mark_user_deleted(&mut self, deleted_at: DateTime<Utc>) -> Result<(), RepositoryError>;
    async fn owned_community_ids(&mut self) -> Result<Vec<CommunityId>, RepositoryError>;
    async fn commit(self: Box<Self>) -> Result<(), RepositoryError>;
}

#[async_trait::async_trait]
pub trait CommunityTransaction: Send {
    async fn load_community(&mut self) -> Result<Community, RepositoryError>;
    async fn lock_pair(&mut self, a: &UserId, b: &UserId) -> Result<(), RepositoryError>;
    async fn block_relation(
        &mut self,
        viewer: &UserId,
        other: &UserId,
    ) -> Result<BlockRelation, RepositoryError>;
    async fn insert_role(&mut self, role: &CommunityRole) -> Result<(), RepositoryError>;
    async fn update_role(
        &mut self,
        role_id: Uuid,
        name: Option<&str>,
        color: Option<&str>,
        permissions: crate::community::role::Permissions,
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
    async fn update_community_details(
        &mut self,
        name: &str,
        description: Option<&str>,
    ) -> Result<bool, RepositoryError>;
    async fn update_community_owner(&mut self, owner_id: &UserId) -> Result<bool, RepositoryError>;
    async fn find_longest_member_excluding(&mut self, exclude: &UserId) -> Result<Option<UserId>, RepositoryError>;
    async fn find_default_channel_id(&mut self) -> Result<Option<ChannelId>, RepositoryError>;
    async fn list_text_channel_member_ids(&mut self, channel_id: &ChannelId) -> Result<Vec<UserId>, RepositoryError>;
    async fn insert_system_message(&mut self, message: &Message) -> Result<(), RepositoryError>;
    async fn redeem_invite_link(
        &mut self,
        token: &InviteToken,
        user_id: &UserId,
    ) -> Result<InviteJoinOutcome, RepositoryError>;
    async fn create_invite_link(&mut self, link: &InviteLink) -> Result<(), RepositoryError>;
    async fn find_primary_invite_link(&mut self) -> Result<Option<InviteLink>, RepositoryError>;
    async fn revoke_invite_link(
        &mut self,
        token: &InviteToken,
        requester: &UserId,
        any_creator: bool,
    ) -> Result<Option<InviteLink>, RepositoryError>;
    async fn insert_dm_message(&mut self, message: &DmMessage) -> Result<(), RepositoryError>;
    async fn insert_community(&mut self, community: &Community) -> Result<(), RepositoryError>;
    async fn is_user_deleted(&mut self, user_id: &UserId) -> Result<bool, RepositoryError>;
    async fn lock_user(&mut self, user_id: &UserId) -> Result<(), RepositoryError>;
    async fn accept_invite(
        &mut self,
        invite_message_id: &DmMessageId,
        user_id: &UserId,
    ) -> Result<bool, RepositoryError>;
    async fn delete_community_by_owner(&mut self, owner_id: &UserId) -> Result<Option<Vec<String>>, RepositoryError>;
    async fn save_events(&mut self, events: &[MessagingEvent], request_id: Option<&str>) -> Result<(), RepositoryError>;
    async fn commit(self: Box<Self>) -> Result<(), RepositoryError>;
}
