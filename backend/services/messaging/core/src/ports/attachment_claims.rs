use async_trait::async_trait;
use domain::{attachment::AttachmentLink, sticker::StickerSnapshot, user::id::UserId};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum AttachmentClaimError {
    #[error("Attachment service unavailable: {0}")]
    Unavailable(String),
    #[error("Invalid attachments")]
    Invalid,
}

#[async_trait]
pub trait AttachmentClaims {
    async fn claim(
        &self,
        ids: &[Uuid],
        owner_id: &UserId,
        claim_token: Uuid,
    ) -> Result<Vec<AttachmentLink>, AttachmentClaimError>;

    async fn release(&self, ids: &[Uuid], claim_token: Uuid) -> Result<(), AttachmentClaimError>;

    async fn lookup_unclaimed(
        &self,
        id: Uuid,
        owner_id: &UserId,
    ) -> Result<Option<AttachmentLink>, AttachmentClaimError>;

    async fn lookup_sticker(&self, id: Uuid) -> Result<Option<StickerSnapshot>, AttachmentClaimError>;
}
