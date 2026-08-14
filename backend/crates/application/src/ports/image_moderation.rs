use async_trait::async_trait;

#[derive(Debug, thiserror::Error)]
pub enum ImageModerationError {
    #[error("Moderation service unavailable: {0}")]
    Unavailable(String),
    #[error("Image rejected by content moderation")]
    Rejected,
}

#[async_trait]
pub trait ImageModeration {
    async fn ensure_allowed(&self, image: &[u8]) -> Result<(), ImageModerationError>;
}
