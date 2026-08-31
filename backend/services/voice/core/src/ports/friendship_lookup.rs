use async_trait::async_trait;
use domain::user::id::UserId;

#[derive(Debug, thiserror::Error)]
pub enum FriendshipLookupError {
    #[error("Friendship lookup unavailable: {0}")]
    Unavailable(String),
}

#[async_trait]
pub trait FriendshipLookup {
    async fn can_call(&self, caller: &UserId, callee: &UserId) -> Result<bool, FriendshipLookupError>;
}
