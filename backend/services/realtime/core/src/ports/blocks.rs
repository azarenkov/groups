use async_trait::async_trait;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum BlockLookupError {
    #[error("Block lookup unavailable: {0}")]
    Unavailable(String),
}

#[derive(Debug, Default, Clone)]
pub struct BlockSets {
    pub blocking: Vec<Uuid>,
    pub blocked_by: Vec<Uuid>,
}

#[async_trait]
pub trait BlockDirectory: Send + Sync {
    async fn block_relations(&self, user_id: Uuid, candidates: &[Uuid]) -> Result<BlockSets, BlockLookupError>;
}
