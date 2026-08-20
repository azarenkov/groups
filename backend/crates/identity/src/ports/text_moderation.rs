use async_trait::async_trait;

#[derive(Debug, thiserror::Error)]
pub enum TextModerationError {
    #[error("Moderation service unavailable: {0}")]
    Unavailable(String),
    #[error("Text rejected by content moderation")]
    Rejected,
}

#[async_trait]
pub trait TextModeration {
    async fn ensure_allowed(&self, text: &str) -> Result<(), TextModerationError>;
}
