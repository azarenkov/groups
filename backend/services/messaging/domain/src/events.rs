use crate::{channel::events::ChannelEvent, dm_message::events::DmMessageDomainEvent, friendship::events::FriendshipDomainEvent, message::events::MessageDomainEvent, community::events::CommunityEvent};

#[derive(Debug)]
pub enum MessagingEvent {
    Channel(ChannelEvent),
    Community(CommunityEvent),
    Message(MessageDomainEvent),
    DmMessage(DmMessageDomainEvent),
    Friendship(FriendshipDomainEvent),
}

impl From<ChannelEvent> for MessagingEvent {
    fn from(event: ChannelEvent) -> Self {
        Self::Channel(event)
    }
}

impl From<CommunityEvent> for MessagingEvent {
    fn from(event: CommunityEvent) -> Self {
        Self::Community(event)
    }
}

impl From<MessageDomainEvent> for MessagingEvent {
    fn from(event: MessageDomainEvent) -> Self {
        Self::Message(event)
    }
}

impl From<DmMessageDomainEvent> for MessagingEvent {
    fn from(event: DmMessageDomainEvent) -> Self {
        Self::DmMessage(event)
    }
}

impl From<FriendshipDomainEvent> for MessagingEvent {
    fn from(event: FriendshipDomainEvent) -> Self {
        Self::Friendship(event)
    }
}
