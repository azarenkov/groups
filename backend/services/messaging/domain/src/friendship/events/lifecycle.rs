use chrono::{DateTime, Utc};
use uuid::Uuid;

use domain::user::id::UserId;

#[derive(Debug)]
pub struct FriendRequestSent {
    pub id: Uuid,
    pub sender_id: UserId,
    pub receiver_id: UserId,
    pub occurred_at: DateTime<Utc>,
}

impl FriendRequestSent {
    pub fn record(sender_id: UserId, receiver_id: UserId, now: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4(),
            sender_id,
            receiver_id,
            occurred_at: now,
        }
    }
}

#[derive(Debug)]
pub struct FriendRequestAccepted {
    pub id: Uuid,
    pub accepter_id: UserId,
    pub sender_id: UserId,
    pub occurred_at: DateTime<Utc>,
}

impl FriendRequestAccepted {
    pub fn record(accepter_id: UserId, sender_id: UserId, now: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4(),
            accepter_id,
            sender_id,
            occurred_at: now,
        }
    }
}

#[derive(Debug)]
pub struct FriendshipRemoved {
    pub id: Uuid,
    pub user_id: UserId,
    pub other_id: UserId,
    pub occurred_at: DateTime<Utc>,
}

impl FriendshipRemoved {
    pub fn record(user_id: UserId, other_id: UserId, now: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            other_id,
            occurred_at: now,
        }
    }
}

#[derive(Debug)]
pub struct UserBlocked {
    pub id: Uuid,
    pub blocker_id: UserId,
    pub blocked_id: UserId,
    pub occurred_at: DateTime<Utc>,
}

impl UserBlocked {
    pub fn record(blocker_id: UserId, blocked_id: UserId, now: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4(),
            blocker_id,
            blocked_id,
            occurred_at: now,
        }
    }
}

#[derive(Debug)]
pub struct UserUnblocked {
    pub id: Uuid,
    pub blocker_id: UserId,
    pub blocked_id: UserId,
    pub occurred_at: DateTime<Utc>,
}

impl UserUnblocked {
    pub fn record(blocker_id: UserId, blocked_id: UserId, now: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4(),
            blocker_id,
            blocked_id,
            occurred_at: now,
        }
    }
}
