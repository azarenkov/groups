use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use domain::user::id::UserId;
use ports::repository::RepositoryError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookJoinOutcome {
    Joined,
    NotCallRoom,
    EvictionPending,
    CallFull,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinOutcome {
    Joined,
    NotParticipant,
    CapExceeded,
}

#[derive(Debug, Clone)]
pub enum AddParticipantsOutcome {
    Added(Vec<UserId>),
    CapExceeded,
    InviterLeft,
}

#[derive(Debug, Clone)]
pub struct PendingEviction {
    pub call_id: Uuid,
    pub user_id: UserId,
    pub attempts: i32,
}

#[async_trait]
pub trait CallRepository {
    async fn start_call(
        &self,
        call_id: Uuid,
        caller: &UserId,
        invitees: &[UserId],
        video: bool,
        now: DateTime<Utc>,
        request_id: Option<String>,
    ) -> Result<(), RepositoryError>;

    async fn call_video(&self, call_id: Uuid) -> Result<bool, RepositoryError>;

    async fn respond(
        &self,
        call_id: Uuid,
        user_id: &UserId,
        accept: bool,
        now: DateTime<Utc>,
        request_id: Option<String>,
    ) -> Result<bool, RepositoryError>;

    async fn add_participants(
        &self,
        call_id: Uuid,
        inviter: &UserId,
        invitees: &[UserId],
        max_participants: usize,
        now: DateTime<Utc>,
        request_id: Option<String>,
    ) -> Result<AddParticipantsOutcome, RepositoryError>;

    async fn leave(
        &self,
        call_id: Uuid,
        user_id: &UserId,
        now: DateTime<Utc>,
        request_id: Option<String>,
    ) -> Result<(), RepositoryError>;

    async fn leave_many(
        &self,
        call_id: Uuid,
        user_ids: &[UserId],
        now: DateTime<Utc>,
        request_id: Option<String>,
    ) -> Result<(), RepositoryError>;

    async fn evict_many(
        &self,
        call_id: Uuid,
        user_ids: &[UserId],
        now: DateTime<Utc>,
        request_id: Option<String>,
    ) -> Result<(), RepositoryError>;

    async fn claim_due_evictions(
        &self,
        now: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<PendingEviction>, RepositoryError>;

    async fn eviction_done(&self, call_id: Uuid, user_id: &UserId) -> Result<(), RepositoryError>;

    async fn eviction_failed(
        &self,
        call_id: Uuid,
        user_id: &UserId,
        next_attempt_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError>;

    async fn is_active_participant(
        &self,
        call_id: Uuid,
        user_id: &UserId,
    ) -> Result<bool, RepositoryError>;

    async fn is_joined_participant(
        &self,
        call_id: Uuid,
        user_id: &UserId,
    ) -> Result<bool, RepositoryError>;

    async fn list_active(
        &self,
        call_id: Uuid,
    ) -> Result<Vec<CallParticipantView>, RepositoryError>;

    async fn active_participant_ids(&self, call_id: Uuid) -> Result<Vec<UserId>, RepositoryError>;

    async fn active_call_for(&self, user_id: &UserId) -> Result<Option<Uuid>, RepositoryError>;

    async fn joined_at(
        &self,
        call_id: Uuid,
        user_id: &UserId,
    ) -> Result<Option<DateTime<Utc>>, RepositoryError>;

    async fn active_call_between(
        &self,
        user_a: &UserId,
        user_b: &UserId,
    ) -> Result<Option<Uuid>, RepositoryError>;

    async fn shared_active_calls(
        &self,
        user_a: &UserId,
        user_b: &UserId,
    ) -> Result<Vec<Uuid>, RepositoryError>;

    async fn join(
        &self,
        call_id: Uuid,
        user_id: &UserId,
        max_participants: usize,
        now: DateTime<Utc>,
    ) -> Result<JoinOutcome, RepositoryError>;

    async fn is_call_room(&self, call_id: Uuid) -> Result<bool, RepositoryError>;

    async fn pair_outcome(
        &self,
        call_id: Uuid,
        caller: &UserId,
        callee: &UserId,
    ) -> Result<CallPairOutcome, RepositoryError>;

    async fn ended_call(&self, call_id: Uuid) -> Result<Option<EndedCall>, RepositoryError>;

    async fn webhook_joined(
        &self,
        call_id: Uuid,
        user_id: &UserId,
        session_id: &str,
        max_participants: usize,
        now: DateTime<Utc>,
        request_id: Option<String>,
    ) -> Result<WebhookJoinOutcome, RepositoryError>;

    async fn webhook_left(
        &self,
        call_id: Uuid,
        user_id: &UserId,
        session_id: &str,
        now: DateTime<Utc>,
        request_id: Option<String>,
    ) -> Result<bool, RepositoryError>;

    async fn webhook_dropped(
        &self,
        call_id: Uuid,
        user_id: &UserId,
        session_id: &str,
        now: DateTime<Utc>,
    ) -> Result<bool, RepositoryError>;

    async fn expire_reconnects(
        &self,
        now: DateTime<Utc>,
        limit: i64,
        request_id: Option<String>,
    ) -> Result<usize, RepositoryError>;
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

#[derive(Debug, Clone)]
pub struct CallPairOutcome {
    pub participated: bool,
    pub caller_in_call: bool,
    pub caller_left_at: Option<DateTime<Utc>>,
    pub callee_left_at: Option<DateTime<Utc>>,
    pub callee_invited_by_caller: bool,
    pub callee_pending: bool,
    pub callee_answered_at: Option<DateTime<Utc>>,
    pub callee_declined: bool,
}

#[derive(Debug, Clone)]
pub struct CallParticipantView {
    pub user_id: UserId,
    pub user_name: String,
}
