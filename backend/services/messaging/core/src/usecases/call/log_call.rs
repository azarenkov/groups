use std::sync::Arc;

use chrono::{DateTime, Utc};
use messaging_domain::{dm_message::DmMessage, dm_message::call::CallStatus, dm_message::unit_of_work::DmUnitOfWork, events::MessagingEvent, message::MessageError, message::content::MessageContent};
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
    #[error("Call verification unavailable: {0}")]
    Lookup(#[from] CallParticipationError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallInvitation {
    pub inviter: UserId,
    pub invitee: UserId,
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

    pub async fn execute(&self, call_id: Uuid, ended_at: DateTime<Utc>, invitations: &[CallInvitation]) -> Result<(), LogCallError> {
        for invitation in invitations {
            self.log(call_id, ended_at, invitation).await?;
        }
        Ok(())
    }

    async fn log(&self, call_id: Uuid, ended_at: DateTime<Utc>, invitation: &CallInvitation) -> Result<(), LogCallError> {
        let CallInvitation { inviter, invitee } = *invitation;
        if inviter == invitee {
            return Ok(());
        }
        let outcome: CallOutcome = self.call_participation.call_outcome(call_id, &inviter, &invitee).await?;
        if !outcome.participated || !outcome.callee_invited_by_caller || outcome.caller_in_call || outcome.callee_pending {
            return Ok(());
        }
        let status = CallStatus::resolve(
            outcome.callee_answered_at,
            CallStatus::conversation_end(outcome.caller_left_at, outcome.callee_left_at, ended_at),
            outcome.callee_declined,
        );
        let content = MessageContent::new("call".to_string())?;
        let mut tx = self.unit_of_work.begin().await?;
        tx.lock_pair(&inviter, &invitee).await?;
        match tx.consume_call_log(call_id, &inviter, &invitee).await {
            Err(RepositoryError::AlreadyExists) => return Ok(()),
            result => result?,
        };
        if CallStatus::predates(ended_at, tx.last_cleared_at(&inviter, &invitee).await?) {
            return Ok(());
        }
        let recipient_read_at = tx.last_read_at(&invitee, &inviter).await?;
        let mut message = DmMessage::create_call(inviter, invitee, call_id, status, content, ended_at, recipient_read_at);
        let events: Vec<MessagingEvent> = message.pull_events().into_iter().map(MessagingEvent::from).collect();
        tx.insert_message(&message, &[]).await?;
        tx.save_events(&events, None).await?;
        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::Mutex;

    use async_trait::async_trait;
    use domain::attachment::AttachmentLink;
    use messaging_domain::{dm_message::id::DmMessageId, dm_message::unit_of_work::DmTransaction, friendship::block::BlockRelation};

    use super::*;

    #[derive(Default)]
    struct Store {
        consumed: Mutex<HashSet<(Uuid, UserId, UserId)>>,
        logged: Mutex<Vec<(UserId, UserId, String)>>,
        created: Mutex<Vec<DateTime<Utc>>>,
        cleared: Mutex<Option<DateTime<Utc>>>,
    }

    struct Uow(Arc<Store>);

    struct Tx {
        store: Arc<Store>,
        logged: Vec<(UserId, UserId, String)>,
    }

    #[async_trait]
    impl DmUnitOfWork for Uow {
        async fn begin(&self) -> Result<Box<dyn DmTransaction>, RepositoryError> {
            Ok(Box::new(Tx { store: self.0.clone(), logged: Vec::new() }))
        }
    }

    #[async_trait]
    impl DmTransaction for Tx {
        async fn lock_pair(&mut self, _: &UserId, _: &UserId) -> Result<(), RepositoryError> {
            Ok(())
        }
        async fn block_relation(&mut self, _: &UserId, _: &UserId) -> Result<BlockRelation, RepositoryError> {
            unimplemented!()
        }
        async fn users_hiding(&mut self, _: Uuid) -> Result<Vec<UserId>, RepositoryError> {
            unimplemented!()
        }
        async fn reply_target_hidden_for(&mut self, _: Uuid, _: &UserId, _: &UserId) -> Result<Option<Vec<UserId>>, RepositoryError> {
            unimplemented!()
        }
        async fn insert_message(&mut self, message: &DmMessage, _: &[AttachmentLink]) -> Result<(), RepositoryError> {
            self.logged.push((*message.sender_id(), *message.recipient_id(), message.call_status().unwrap_or_default().to_string()));
            self.store.created.lock().unwrap().push(message.created_at());
            Ok(())
        }
        async fn last_read_at(&mut self, _: &UserId, _: &UserId) -> Result<Option<DateTime<Utc>>, RepositoryError> {
            Ok(None)
        }
        async fn last_cleared_at(&mut self, _: &UserId, _: &UserId) -> Result<Option<DateTime<Utc>>, RepositoryError> {
            Ok(*self.store.cleared.lock().unwrap())
        }
        async fn consume_call_log(&mut self, call_id: Uuid, sender: &UserId, recipient: &UserId) -> Result<(), RepositoryError> {
            if self.store.consumed.lock().unwrap().insert((call_id, *sender, *recipient)) {
                Ok(())
            } else {
                Err(RepositoryError::AlreadyExists)
            }
        }
        async fn update_message(&mut self, _: &DmMessage, _: &[AttachmentLink], _: &[Uuid]) -> Result<(), RepositoryError> {
            unimplemented!()
        }
        async fn delete_message(&mut self, _: &DmMessageId) -> Result<(), RepositoryError> {
            unimplemented!()
        }
        async fn save_events(&mut self, _: &[MessagingEvent], _: Option<&str>) -> Result<(), RepositoryError> {
            Ok(())
        }
        async fn commit(self: Box<Self>) -> Result<(), RepositoryError> {
            self.store.logged.lock().unwrap().extend(self.logged);
            Ok(())
        }
    }

    struct Outcomes(CallOutcome);

    #[async_trait]
    impl CallParticipationLookup for Outcomes {
        async fn call_outcome(&self, _: Uuid, _: &UserId, _: &UserId) -> Result<CallOutcome, CallParticipationError> {
            Ok(self.0.clone())
        }
    }

    fn ended(declined: bool) -> CallOutcome {
        CallOutcome {
            participated: true,
            caller_in_call: false,
            caller_left_at: None,
            callee_left_at: None,
            callee_invited_by_caller: true,
            callee_pending: false,
            callee_answered_at: None,
            callee_declined: declined,
        }
    }

    fn usecase(outcome: CallOutcome) -> (Arc<Store>, LogCall<Uow, Outcomes>) {
        let store = Arc::new(Store::default());
        (store.clone(), LogCall::new(Arc::new(Uow(store)), Arc::new(Outcomes(outcome))))
    }

    #[tokio::test]
    async fn an_unanswered_ended_call_leaves_one_missed_card_dated_when_the_call_ended() {
        let (store, log_call) = usecase(ended(false));
        let (caller, callee) = (UserId::new(Uuid::new_v4()), UserId::new(Uuid::new_v4()));
        let call_id = Uuid::new_v4();
        let invitations = [CallInvitation { inviter: caller, invitee: callee }];
        let ended_at = DateTime::from_timestamp(1_800_000_000, 0).unwrap();
        log_call.execute(call_id, ended_at, &invitations).await.unwrap();
        log_call.execute(call_id, ended_at, &invitations).await.unwrap();
        assert_eq!(*store.logged.lock().unwrap(), vec![(caller, callee, "missed".to_string())]);
        assert_eq!(store.created.lock().unwrap()[0], ended_at);
    }

    #[tokio::test]
    async fn a_declined_call_is_logged_as_declined_and_unverified_calls_are_skipped() {
        let (store, log_call) = usecase(ended(true));
        let (caller, callee) = (UserId::new(Uuid::new_v4()), UserId::new(Uuid::new_v4()));
        log_call.execute(Uuid::new_v4(), Utc::now(), &[CallInvitation { inviter: caller, invitee: callee }]).await.unwrap();
        assert_eq!(*store.logged.lock().unwrap(), vec![(caller, callee, "declined".to_string())]);

        let (store, log_call) = usecase(CallOutcome { callee_pending: true, ..ended(false) });
        log_call.execute(Uuid::new_v4(), Utc::now(), &[CallInvitation { inviter: caller, invitee: callee }]).await.unwrap();
        assert!(store.logged.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_call_that_ended_before_the_conversation_was_cleared_leaves_no_card() {
        let (store, log_call) = usecase(ended(false));
        let ended_at = DateTime::from_timestamp(1_800_000_000, 0).unwrap();
        *store.cleared.lock().unwrap() = Some(ended_at + chrono::Duration::seconds(5));
        let (caller, callee) = (UserId::new(Uuid::new_v4()), UserId::new(Uuid::new_v4()));
        log_call.execute(Uuid::new_v4(), ended_at, &[CallInvitation { inviter: caller, invitee: callee }]).await.unwrap();
        assert!(store.logged.lock().unwrap().is_empty());
    }
}
