use async_trait::async_trait;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum AttachmentLinksLookupError {
    #[error("Links lookup unavailable: {0}")]
    Unavailable(String),
}

#[async_trait]
pub trait AttachmentLinksLookup {
    async fn linked_subset(
        &self,
        ids: &[Uuid],
    ) -> Result<Vec<Uuid>, AttachmentLinksLookupError>;
}
