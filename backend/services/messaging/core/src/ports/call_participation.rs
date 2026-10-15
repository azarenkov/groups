use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CallOutcome {
    pub participated: bool,
    pub caller_in_call: bool,
    pub caller_left_at: Option<DateTime<Utc>>,
    pub callee_left_at: Option<DateTime<Utc>>,
    pub callee_invited_by_caller: bool,
    pub callee_pending: bool,
    pub callee_answered_at: Option<DateTime<Utc>>,
    pub callee_declined: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallInvitation {
    pub inviter: UserId,
    pub invitee: UserId,
}

#[derive(Debug, Clone)]
pub struct EndedCall {
    pub ended_at: DateTime<Utc>,
    pub invitations: Vec<CallInvitation>,
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

    async fn ended_call(&self, call_id: Uuid) -> Result<Option<EndedCall>, CallParticipationError>;
}
