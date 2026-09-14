use async_trait::async_trait;
use domain::user::id::UserId;

#[derive(Debug, thiserror::Error)]
pub enum BlockLookupError {
    #[error("Block lookup unavailable: {0}")]
    Unavailable(String),
}

#[async_trait]
pub trait BlockLookup: Send + Sync {
    async fn blocked_by(&self, viewer: &UserId, target: &UserId) -> Result<bool, BlockLookupError>;
}
