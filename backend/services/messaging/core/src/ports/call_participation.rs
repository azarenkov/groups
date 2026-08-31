use async_trait::async_trait;
use domain::user::id::UserId;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum CallParticipationError {
    #[error("Call participation lookup unavailable: {0}")]
    Unavailable(String),
}

#[async_trait]
pub trait CallParticipationLookup: Send + Sync {
    async fn both_participated(
        &self,
        call_id: Uuid,
        a: &UserId,
        b: &UserId,
    ) -> Result<bool, CallParticipationError>;
}
