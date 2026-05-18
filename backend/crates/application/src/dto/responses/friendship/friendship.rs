use chrono::{DateTime, Utc};
use domain::friendship::{status::FriendshipStatus, view::FriendshipView};
use uuid::Uuid;

#[derive(Debug)]
pub struct FriendshipDto {
    pub friend_id: Uuid,
    pub friend_name: String,
    pub status: FriendshipStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<FriendshipView> for FriendshipDto {
    fn from(value: FriendshipView) -> Self {
        Self {
            friend_id: value.friend_id.uuid(),
            friend_name: value.friend_name,
            status: value.status,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
