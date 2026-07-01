use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{shared::repository::error::RepositoryError, user::id::UserId};

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
}
