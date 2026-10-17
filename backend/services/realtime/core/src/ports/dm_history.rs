use async_trait::async_trait;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum DmHistoryLookupError {
    #[error("DM history lookup unavailable: {0}")]
    Unavailable(String),
}

#[async_trait]
pub trait DmHistory: Send + Sync {
    async fn dm_peers(&self, user_id: Uuid, candidates: &[Uuid]) -> Result<Vec<Uuid>, DmHistoryLookupError>;
}
