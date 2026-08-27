use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use domain::{shared::repository::error::RepositoryError, user::id::UserId};

#[async_trait]
pub trait CallRepository {
    async fn start_call(
        &self,
        call_id: Uuid,
        caller: &UserId,
        invitees: &[UserId],
        now: DateTime<Utc>,
        request_id: Option<String>,
    ) -> Result<(), RepositoryError>;

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
        now: DateTime<Utc>,
        request_id: Option<String>,
    ) -> Result<(), RepositoryError>;

    async fn leave(
        &self,
        call_id: Uuid,
        user_id: &UserId,
        now: DateTime<Utc>,
        request_id: Option<String>,
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

    async fn active_call_for(&self, user_id: &UserId) -> Result<Option<Uuid>, RepositoryError>;

    async fn active_call_between(
        &self,
        user_a: &UserId,
        user_b: &UserId,
    ) -> Result<Option<Uuid>, RepositoryError>;

    async fn join(
        &self,
        call_id: Uuid,
        user_id: &UserId,
        now: DateTime<Utc>,
    ) -> Result<bool, RepositoryError>;

    async fn is_call_room(&self, call_id: Uuid) -> Result<bool, RepositoryError>;

    async fn webhook_joined(
        &self,
        call_id: Uuid,
        user_id: &UserId,
        session_id: &str,
        now: DateTime<Utc>,
        request_id: Option<String>,
    ) -> Result<bool, RepositoryError>;

    async fn webhook_left(
        &self,
        call_id: Uuid,
        user_id: &UserId,
        session_id: &str,
        now: DateTime<Utc>,
        request_id: Option<String>,
    ) -> Result<bool, RepositoryError>;
}

#[derive(Debug, Clone)]
pub struct CallParticipantView {
    pub user_id: UserId,
    pub user_name: String,
}
