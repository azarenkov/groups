use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{events::MessagingEvent, dm_message::DmMessage, dm_message::unit_of_work::DmUnitOfWork, message::MessageError, message::content::MessageContent};
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use uuid::Uuid;

use crate::ports::call_participation::{CallParticipationError, CallParticipationLookup};

#[derive(Debug, thiserror::Error)]
pub enum LogCallError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Message error: {0}")]
    Domain(#[from] MessageError),
    #[error("Invalid call status")]
    InvalidStatus,
    #[error("Call could not be verified")]
    CallNotVerified,
    #[error("Call verification unavailable: {0}")]
    Lookup(#[from] CallParticipationError),
}

pub struct LogCall<W, L>
where
    W: DmUnitOfWork,
    L: CallParticipationLookup,
{
    unit_of_work: Arc<W>,
    call_participation: Arc<L>,
}

impl<W, L> LogCall<W, L>
where
    W: DmUnitOfWork,
    L: CallParticipationLookup,
{
    pub fn new(unit_of_work: Arc<W>, call_participation: Arc<L>) -> Self {
        Self {
            unit_of_work,
            call_participation,
        }
    }

    pub async fn execute(
        &self,
        sender_id: UserId,
        recipient_id: Uuid,
        call_id: Uuid,
        status: String,
        duration_seconds: Option<i32>,
    ) -> Result<(), LogCallError> {
        if !matches!(status.as_str(), "completed" | "declined" | "missed") {
            return Err(LogCallError::InvalidStatus);
        }
        let recipient_id = UserId::new(recipient_id);
        if !self
            .call_participation
            .both_participated(call_id, &sender_id, &recipient_id)
            .await?
        {
            return Err(LogCallError::CallNotVerified);
        }
        let duration_seconds = duration_seconds.map(|d| d.max(0));
        let content = MessageContent::new("call".to_string())?;
        let now = Utc::now();
        let mut message =
            DmMessage::create_call(sender_id, recipient_id, status, duration_seconds, content, now);
        let events = message.pull_events();
        let events: Vec<MessagingEvent> = events.into_iter().map(MessagingEvent::from).collect();
        let mut tx = self.unit_of_work.begin().await?;
        tx.insert_message(&message, &[]).await?;
        tx.save_events(&events, None).await?;
        tx.commit().await?;
        Ok(())
    }
}
