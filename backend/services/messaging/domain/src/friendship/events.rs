pub mod lifecycle;

use chrono::{DateTime, Utc};

use crate::{friendship::events::lifecycle::FriendRequestAccepted, friendship::events::lifecycle::FriendRequestSent, friendship::events::lifecycle::FriendshipRemoved, friendship::events::lifecycle::UserBlocked, friendship::events::lifecycle::UserUnblocked};
use domain::user::id::UserId;

#[derive(Debug)]
pub enum FriendshipDomainEvent {
    RequestSent(FriendRequestSent),
    RequestAccepted(FriendRequestAccepted),
    Removed(FriendshipRemoved),
    Blocked(UserBlocked),
    Unblocked(UserUnblocked),
}

impl FriendshipDomainEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::RequestSent(_) => "request_sent",
            Self::RequestAccepted(_) => "request_accepted",
            Self::Removed(_) => "removed",
            Self::Blocked(_) => "blocked",
            Self::Unblocked(_) => "unblocked",
        }
    }

    pub fn aggregate_user_id(&self) -> UserId {
        match self {
            Self::RequestSent(e) => e.sender_id,
            Self::RequestAccepted(e) => e.accepter_id,
            Self::Removed(e) => e.user_id,
            Self::Blocked(e) => e.blocker_id,
            Self::Unblocked(e) => e.blocker_id,
        }
    }

    pub fn parent_user_id(&self) -> UserId {
        match self {
            Self::RequestSent(e) => e.receiver_id,
            Self::RequestAccepted(e) => e.sender_id,
            Self::Removed(e) => e.other_id,
            Self::Blocked(e) => e.blocked_id,
            Self::Unblocked(e) => e.blocked_id,
        }
    }

    pub fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            Self::RequestSent(e) => e.occurred_at,
            Self::RequestAccepted(e) => e.occurred_at,
            Self::Removed(e) => e.occurred_at,
            Self::Blocked(e) => e.occurred_at,
            Self::Unblocked(e) => e.occurred_at,
        }
    }
}
