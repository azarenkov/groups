pub mod events;
pub mod repository;
pub mod status;
pub mod view;

use chrono::{DateTime, Utc};

use crate::{friendship::events::FriendshipDomainEvent, friendship::events::lifecycle::FriendRequestAccepted, friendship::events::lifecycle::FriendRequestSent, friendship::events::lifecycle::FriendshipRemoved, friendship::status::FriendshipStatus};
use domain::user::id::UserId;

#[derive(Debug, thiserror::Error)]
pub enum FriendshipError {
    #[error("Cannot befriend yourself")]
    SelfFriendship,
    #[error("Friend request already exists")]
    AlreadyRequested,
    #[error("Already friends")]
    AlreadyFriends,
    #[error("No pending request from this user")]
    NoPendingRequest,
    #[error("Not friends with this user")]
    NotFriends,
}

#[derive(Debug, Clone)]
pub struct Friendship {
    pub user_id: UserId,
    pub friend_id: UserId,
    pub status: FriendshipStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Friendship {
    pub fn build_request_pair(
        sender: UserId,
        receiver: UserId,
        now: DateTime<Utc>,
    ) -> Result<(Self, Self, Vec<FriendshipDomainEvent>), FriendshipError> {
        if sender == receiver {
            return Err(FriendshipError::SelfFriendship);
        }
        let outgoing = Self {
            user_id: sender,
            friend_id: receiver,
            status: FriendshipStatus::PendingOutgoing,
            created_at: now,
            updated_at: now,
        };
        let incoming = Self {
            user_id: receiver,
            friend_id: sender,
            status: FriendshipStatus::PendingIncoming,
            created_at: now,
            updated_at: now,
        };
        let events = vec![FriendshipDomainEvent::RequestSent(
            FriendRequestSent::record(sender, receiver, now),
        )];
        Ok((outgoing, incoming, events))
    }

    pub fn accept_pair(
        accepter: UserId,
        sender: UserId,
        now: DateTime<Utc>,
    ) -> Vec<FriendshipDomainEvent> {
        vec![FriendshipDomainEvent::RequestAccepted(
            FriendRequestAccepted::record(accepter, sender, now),
        )]
    }

    pub fn removal_events(
        user: UserId,
        other: UserId,
        now: DateTime<Utc>,
    ) -> Vec<FriendshipDomainEvent> {
        vec![FriendshipDomainEvent::Removed(FriendshipRemoved::record(
            user, other, now,
        ))]
    }
}
pub mod unit_of_work;
