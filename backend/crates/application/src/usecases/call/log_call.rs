use std::sync::Arc;

use chrono::Utc;
use domain::{
    dm_message::{DmMessage, repository::DmMessageRepository},
    friendship::{repository::FriendshipRepository, status::FriendshipStatus},
    message::{MessageError, content::MessageContent},
    shared::{
        outbox::{
            outbox_record::OutboxRecord,
            to_outbox::{OutboxError, ToOutboxRecord},
        },
        repository::error::RepositoryError,
    },
    user::id::UserId,
};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum LogCallError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Message error: {0}")]
    Domain(#[from] MessageError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
    #[error("Not friends with this user")]
    NotFriends,
}

pub struct LogCall<M, F>
where
    M: DmMessageRepository,
    F: FriendshipRepository,
{
    dm_repository: Arc<M>,
    friendship_repository: Arc<F>,
}

impl<M, F> LogCall<M, F>
where
    M: DmMessageRepository,
    F: FriendshipRepository,
{
    pub fn new(dm_repository: Arc<M>, friendship_repository: Arc<F>) -> Self {
        Self {
            dm_repository,
            friendship_repository,
        }
    }

    pub async fn execute(
        &self,
        sender_id: UserId,
        recipient_id: Uuid,
        status: String,
        duration_seconds: Option<i32>,
    ) -> Result<(), LogCallError> {
        let recipient_id = UserId::new(recipient_id);

        let pair = self
            .friendship_repository
            .find_pair(&sender_id, &recipient_id)
            .await?;
        if !matches!(pair, Some(ref f) if f.status == FriendshipStatus::Accepted) {
            return Err(LogCallError::NotFriends);
        }

        let content = MessageContent::new("call".to_string())?;
        let now = Utc::now();
        let mut message =
            DmMessage::create_call(sender_id, recipient_id, status, duration_seconds, content, now);
        let records: Vec<OutboxRecord> = message
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(None))
            .collect::<Result<_, _>>()?;
        self.dm_repository
            .save_with_events(&message, &records, &[])
            .await?;
        Ok(())
    }
}
