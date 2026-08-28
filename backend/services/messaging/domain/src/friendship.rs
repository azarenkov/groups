pub mod events;
pub mod pair;
pub mod repository;
pub mod status;
pub mod unit_of_work;
pub mod view;

#[cfg(test)]
mod tests;

use chrono::{DateTime, Utc};

use crate::friendship::status::FriendshipStatus;
use domain::user::id::UserId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FriendshipError {
    #[error("Cannot befriend yourself")]
    SelfFriendship,
    #[error("Friend request already exists")]
    AlreadyRequested,
    #[error("This user already sent you a friend request")]
    IncomingPending,
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
