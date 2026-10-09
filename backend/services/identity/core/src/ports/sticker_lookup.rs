use async_trait::async_trait;
use domain::user::id::UserId;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum StickerLookupError {
    #[error("Sticker lookup unavailable: {0}")]
    Unavailable(String),
}

#[async_trait]
pub trait StickerLookup: Send + Sync {
    async fn usable_sticker(&self, user_id: &UserId, sticker_id: Uuid) -> Result<bool, StickerLookupError>;
}
