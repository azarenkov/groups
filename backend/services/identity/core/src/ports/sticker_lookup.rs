use async_trait::async_trait;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum StickerLookupError {
    #[error("Sticker lookup unavailable: {0}")]
    Unavailable(String),
}

#[async_trait]
pub trait StickerLookup: Send + Sync {
    async fn usable_sticker(&self, sticker_id: Uuid) -> Result<bool, StickerLookupError>;
}
