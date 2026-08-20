use async_trait::async_trait;
use domain::user::id::UserId;

#[derive(Debug, thiserror::Error)]
pub enum FriendshipLookupError {
    #[error("Friendship lookup unavailable: {0}")]
    Unavailable(String),
}

#[async_trait]
pub trait FriendshipLookup {
    async fn are_friends(&self, a: &UserId, b: &UserId) -> Result<bool, FriendshipLookupError>;
}
