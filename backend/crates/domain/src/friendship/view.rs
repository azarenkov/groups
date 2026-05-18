use chrono::{DateTime, Utc};

use crate::{friendship::status::FriendshipStatus, user::id::UserId};

#[derive(Debug, Clone)]
pub struct FriendshipView {
    pub user_id: UserId,
    pub friend_id: UserId,
    pub friend_name: String,
    pub status: FriendshipStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
