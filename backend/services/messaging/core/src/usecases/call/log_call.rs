use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{events::MessagingEvent, dm_message::DmMessage, dm_message::unit_of_work::DmUnitOfWork, message::MessageError, message::content::MessageContent};
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use uuid::Uuid;

use crate::ports::call_participation::{CallOutcome, CallParticipationError, CallParticipationLookup};

#[derive(Debug, thiserror::Error)]
pub enum LogCallError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Message error: {0}")]
    Domain(#[from] MessageError),
    #[error("You cannot log a call with yourself")]
    SelfCall,
    #[error("Call could not be verified")]
    CallNotVerified,
    #[error("This call is already logged")]
    AlreadyLogged,
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
        _status: String,
        _duration_seconds: Option<i32>,
    ) -> Result<(), LogCallError> {
        let recipient_id = UserId::new(recipient_id);
        if sender_id == recipient_id {
            return Err(LogCallError::SelfCall);
        }
        let outcome: CallOutcome = self
            .call_participation
            .call_outcome(call_id, &sender_id, &recipient_id)
            .await?;
        if !outcome.participated
            || !outcome.callee_invited_by_caller
            || outcome.caller_in_call
            || outcome.callee_pending
        {
            return Err(LogCallError::CallNotVerified);
        }
        let now = Utc::now();
        let (status, duration_seconds) = if let Some(answered_at) = outcome.callee_answered_at {
            let ended_at = outcome.caller_left_at.unwrap_or(now);
            let duration =
                (ended_at - answered_at).num_seconds().clamp(0, i64::from(i32::MAX)) as i32;
            ("completed".to_string(), Some(duration))
        } else if outcome.callee_declined {
            ("declined".to_string(), None)
        } else {
            ("missed".to_string(), None)
        };
        let content = MessageContent::new("call".to_string())?;
        let mut message =
            DmMessage::create_call(sender_id, recipient_id, call_id, status, duration_seconds, content, now);
        let mut tx = self.unit_of_work.begin().await?;
        tx.lock_pair(&sender_id, &recipient_id).await?;
        match tx.consume_call_log(call_id, &sender_id, &recipient_id).await {
            Err(RepositoryError::AlreadyExists) => return Err(LogCallError::AlreadyLogged),
            result => result?,
        };
        message.restamp(Utc::now());
        let events: Vec<MessagingEvent> = message.pull_events().into_iter().map(MessagingEvent::from).collect();
        tx.insert_message(&message, &[]).await?;
        tx.save_events(&events, None).await?;
        tx.commit().await?;
        Ok(())
    }
}
