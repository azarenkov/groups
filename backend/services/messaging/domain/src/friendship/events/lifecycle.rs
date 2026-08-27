use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use domain::user::id::UserId;

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
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
