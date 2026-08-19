use async_trait::async_trait;
use domain::{attachment::AttachmentLink, user::id::UserId};
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
    ) -> Result<Vec<AttachmentLink>, AttachmentClaimError>;

    async fn release(&self, ids: &[Uuid]) -> Result<(), AttachmentClaimError>;
}
