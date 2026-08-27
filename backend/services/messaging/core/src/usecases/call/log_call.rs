use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{dm_message::DmMessage, dm_message::unit_of_work::DmUnitOfWork, friendship::repository::FriendshipRepository, friendship::status::FriendshipStatus, message::MessageError, message::content::MessageContent};
use domain::user::id::UserId;
use ports::{outbox::record::OutboxRecord, outbox::to_outbox::OutboxError, outbox::to_outbox::ToOutboxRecord, repository::RepositoryError};
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

pub struct LogCall<W, F>
where
    W: DmUnitOfWork,
    F: FriendshipRepository,
{
    unit_of_work: Arc<W>,
    friendship_repository: Arc<F>,
}

impl<W, F> LogCall<W, F>
where
    W: DmUnitOfWork,
    F: FriendshipRepository,
{
    pub fn new(unit_of_work: Arc<W>, friendship_repository: Arc<F>) -> Self {
        Self {
            unit_of_work,
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
        let status_snapshot = status.clone();
        let mut message =
            DmMessage::create_call(sender_id, recipient_id, status, duration_seconds, content, now);
        let mut records: Vec<OutboxRecord> = message
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(None))
            .collect::<Result<_, _>>()?;
        crate::usecases::enrichment::inject_field(
            &mut records,
            "call_status",
            serde_json::json!(status_snapshot),
        );
        crate::usecases::enrichment::inject_field(
            &mut records,
            "call_duration_seconds",
            serde_json::json!(duration_seconds),
        );
        let mut tx = self.unit_of_work.begin().await?;
        tx.insert_message(&message, &[]).await?;
        tx.save_outbox(&records).await?;
        tx.commit().await?;
        Ok(())
    }
}
