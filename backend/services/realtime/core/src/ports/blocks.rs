use async_trait::async_trait;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum RelationLookupError {
    #[error("Block lookup unavailable: {0}")]
    Unavailable(String),
}

#[derive(Debug, Default, Clone)]
pub struct BlockSets {
    pub blocking: Vec<Uuid>,
    pub blocked_by: Vec<Uuid>,
}

#[async_trait]
pub trait RelationDirectory: Send + Sync {
    async fn block_relations(&self, user_id: Uuid, candidates: &[Uuid]) -> Result<BlockSets, RelationLookupError>;
    async fn dm_peers(&self, user_id: Uuid, candidates: &[Uuid]) -> Result<Vec<Uuid>, RelationLookupError>;
}
