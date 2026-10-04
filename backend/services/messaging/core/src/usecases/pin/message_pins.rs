use std::sync::Arc;

use domain::user::id::UserId;
use messaging_domain::{
    channel::{id::ChannelId, repository::ChannelRepository, visibility::ChannelVisibility},
    community::{id::CommunityId, repository::CommunityRepository, role::Permission, unit_of_work::CommunityUnitOfWork},
    friendship::repository::FriendshipRepository,
    pin::{MessagePinRepository, PinOutcome, PinScope, PinnedMessage},
};
use ports::repository::RepositoryError;
use uuid::Uuid;

use crate::dto::responses::{channel::channel_message::ChannelMessageDto, dm::dm_message::DmMessageDto};

#[derive(Debug, thiserror::Error)]
pub enum MessagePinError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Not a member of the community")]
    NotCommunityMember,
    #[error("Not a member of the channel")]
    NotChannelMember,
    #[error("Channel not found")]
    ChannelNotFound,
    #[error("Channel is not a text channel")]
    NotTextChannel,
    #[error("Not allowed to pin messages here")]
    NotAllowed,
    #[error("This message cannot be pinned")]
    NotPinnable,
    #[error("Too many pinned messages")]
    LimitReached,
    #[error("Conversation is blocked")]
    Blocked,
    #[error("Cannot pin messages with yourself")]
    SelfConversation,
}

pub struct MessagePins<P, C, S, W, F>
where
    P: MessagePinRepository,
    C: ChannelRepository,
    S: CommunityRepository,
    W: CommunityUnitOfWork,
    F: FriendshipRepository,
{
    pins: Arc<P>,
    channels: Arc<C>,
    communities: Arc<S>,
    unit_of_work: Arc<W>,
    friendships: Arc<F>,
}

impl<P, C, S, W, F> MessagePins<P, C, S, W, F>
where
    P: MessagePinRepository,
    C: ChannelRepository,
    S: CommunityRepository,
    W: CommunityUnitOfWork,
    F: FriendshipRepository,
{
    pub fn new(pins: Arc<P>, channels: Arc<C>, communities: Arc<S>, unit_of_work: Arc<W>, friendships: Arc<F>) -> Self {
        Self {
            pins,
            channels,
            communities,
            unit_of_work,
            friendships,
        }
    }

    pub async fn list_channel(
        &self,
        community_id: Uuid,
        channel_id: Uuid,
        viewer: UserId,
    ) -> Result<Vec<PinnedMessage>, MessagePinError> {
        let channel_id = ChannelId::new(channel_id);
        self.ensure_readable(&CommunityId::new(community_id), &channel_id, &viewer).await?;
        Ok(self.pins.list(PinScope::Channel(channel_id.uuid()), &viewer).await?)
    }

    pub async fn list_dm(&self, viewer: UserId, friend: UserId) -> Result<Vec<PinnedMessage>, MessagePinError> {
        if viewer == friend {
            return Err(MessagePinError::SelfConversation);
        }
        if self.friendships.block_relation(&viewer, &friend).await?.any() {
            return Err(MessagePinError::Blocked);
        }
        Ok(self.pins.list(PinScope::dm(&viewer, &friend), &viewer).await?)
    }

    pub async fn set_in_channel(
        &self,
        community_id: Uuid,
        message: &ChannelMessageDto,
        actor: UserId,
        pinned: bool,
    ) -> Result<(), MessagePinError> {
        if message.kind != "user" {
            return Err(MessagePinError::NotPinnable);
        }
        let community_id = CommunityId::new(community_id);
        let channel_id = ChannelId::new(message.channel_id);
        self.ensure_readable(&community_id, &channel_id, &actor).await?;
        let community = {
            let mut tx = self.unit_of_work.begin(&community_id).await?;
            tx.load_community().await?
        };
        if !community.has(&actor, Permission::ManageMessages) {
            return Err(MessagePinError::NotAllowed);
        }
        let recipients: Vec<Uuid> = self
            .channels
            .list_text_channel_member_ids(&channel_id)
            .await?
            .iter()
            .map(UserId::uuid)
            .collect();
        self.apply(PinScope::Channel(channel_id.uuid()), message.id, &actor, pinned, &recipients)
            .await
    }

    pub async fn set_in_dm(
        &self,
        actor: UserId,
        friend: UserId,
        message: &DmMessageDto,
        pinned: bool,
    ) -> Result<(), MessagePinError> {
        if actor == friend {
            return Err(MessagePinError::SelfConversation);
        }
        if message.call_status.is_some() || message.invite_community_id.is_some() {
            return Err(MessagePinError::NotPinnable);
        }
        if self.friendships.block_relation(&actor, &friend).await?.any() {
            return Err(MessagePinError::Blocked);
        }
        self.apply(
            PinScope::dm(&actor, &friend),
            message.id,
            &actor,
            pinned,
            &[actor.uuid(), friend.uuid()],
        )
        .await
    }

    async fn apply(
        &self,
        scope: PinScope,
        message_id: Uuid,
        actor: &UserId,
        pinned: bool,
        recipients: &[Uuid],
    ) -> Result<(), MessagePinError> {
        if !pinned {
            self.pins.unpin(scope, message_id, actor, recipients).await?;
            return Ok(());
        }
        match self.pins.pin(scope, message_id, actor, recipients).await? {
            PinOutcome::Pinned(_) | PinOutcome::AlreadyPinned => Ok(()),
            PinOutcome::LimitReached => Err(MessagePinError::LimitReached),
        }
    }

    async fn ensure_readable(
        &self,
        community_id: &CommunityId,
        channel_id: &ChannelId,
        viewer: &UserId,
    ) -> Result<(), MessagePinError> {
        if !self.communities.is_community_member(community_id, viewer).await? {
            return Err(MessagePinError::NotCommunityMember);
        }
        let channel = self.channels.find_channel_by_id(channel_id).await.map_err(|e| match e {
            RepositoryError::NotFound => MessagePinError::ChannelNotFound,
            other => MessagePinError::Repository(other),
        })?;
        if channel.community_id() != community_id {
            return Err(MessagePinError::ChannelNotFound);
        }
        if !channel.supports_text() {
            return Err(MessagePinError::NotTextChannel);
        }
        if channel.visibility() == ChannelVisibility::Private
            && !self.channels.is_text_channel_member(channel_id, viewer).await?
        {
            return Err(MessagePinError::NotChannelMember);
        }
        Ok(())
    }
}
