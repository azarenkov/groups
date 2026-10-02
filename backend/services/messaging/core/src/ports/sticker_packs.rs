use async_trait::async_trait;
use domain::user::id::UserId;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StickerPackInfo {
    pub id: Uuid,
    pub owner_id: UserId,
    pub name: String,
}

#[derive(Debug, thiserror::Error)]
#[error("Sticker pack directory unavailable: {0}")]
pub struct StickerPackLookupError(pub String);

#[async_trait]
pub trait StickerPackDirectory {
    async fn find_pack(&self, id: Uuid) -> Result<Option<StickerPackInfo>, StickerPackLookupError>;
}
