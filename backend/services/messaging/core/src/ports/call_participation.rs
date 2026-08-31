use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CallOutcome {
    pub participated: bool,
    pub callee_answered_at: Option<DateTime<Utc>>,
    pub callee_declined: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum CallParticipationError {
    #[error("Call outcome lookup unavailable: {0}")]
    Unavailable(String),
}

#[async_trait]
pub trait CallParticipationLookup: Send + Sync {
    async fn call_outcome(
        &self,
        call_id: Uuid,
        caller: &UserId,
        callee: &UserId,
    ) -> Result<CallOutcome, CallParticipationError>;
}
