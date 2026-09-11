use async_trait::async_trait;
use domain::user::id::UserId;

#[derive(Debug, thiserror::Error)]
pub enum FriendshipLookupError {
    #[error("Friendship lookup unavailable: {0}")]
    Unavailable(String),
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BlockRelation {
    pub blocking: bool,
    pub blocked_by: bool,
}

impl BlockRelation {
    pub fn any(&self) -> bool {
        self.blocking || self.blocked_by
    }
}

#[async_trait]
pub trait FriendshipLookup: Send + Sync {
    async fn can_call(&self, caller: &UserId, callee: &UserId) -> Result<bool, FriendshipLookupError>;
    async fn block_relation(&self, viewer: &UserId, other: &UserId) -> Result<BlockRelation, FriendshipLookupError>;
    async fn is_blocked(&self, a: &UserId, b: &UserId) -> Result<bool, FriendshipLookupError> {
        Ok(self.block_relation(a, b).await?.any())
    }
}
