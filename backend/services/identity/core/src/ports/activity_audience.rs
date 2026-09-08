use async_trait::async_trait;
use domain::user::id::UserId;

#[derive(Debug, thiserror::Error)]
pub enum ActivityAudienceError {
    #[error("Activity audience lookup unavailable: {0}")]
    Unavailable(String),
}

#[async_trait]
pub trait ActivityAudience: Send + Sync {
    async fn visible_to(
        &self,
        viewer: &UserId,
        candidates: &[UserId],
    ) -> Result<Vec<UserId>, ActivityAudienceError>;
}
