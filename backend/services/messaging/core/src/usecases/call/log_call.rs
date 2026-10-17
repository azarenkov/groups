use std::sync::Arc;

use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use messaging_domain::{dm_message::DmMessage, dm_message::call::{CallStatus, conversation_of}, dm_message::unit_of_work::DmUnitOfWork, events::MessagingEvent, message::MessageError, message::content::MessageContent};
use ports::repository::RepositoryError;
use uuid::Uuid;

use crate::ports::call_participation::{CallInvitation, CallOutcome, CallParticipationError, CallParticipationLookup};

#[derive(Debug, thiserror::Error)]
pub enum LogCallError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Message error: {0}")]
    Domain(#[from] MessageError),
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

    pub async fn execute(&self, call_id: Uuid) -> Result<(), LogCallError> {
        let Some(ended) = self.call_participation.ended_call(call_id).await? else {
            return Ok(());
        };
        let mut conversations: Vec<((UserId, UserId), CallInvitation, CallStatus)> = Vec::new();
        for invitation in &ended.invitations {
            let Some(status) = self.resolve(call_id, ended.ended_at, invitation).await? else {
                continue;
            };
            let key = conversation_of(invitation.inviter, invitation.invitee);
            match conversations.iter_mut().find(|(k, _, _)| *k == key) {
                Some(best) if status.outranks(&best.2) => *best = (key, *invitation, status),
                Some(_) => {}
                None => conversations.push((key, *invitation, status)),
            }
        }
        for (_, invitation, status) in conversations {
            self.record(call_id, ended.ended_at, invitation, status).await?;
        }
        Ok(())
    }

    async fn resolve(&self, call_id: Uuid, ended_at: DateTime<Utc>, invitation: &CallInvitation) -> Result<Option<CallStatus>, LogCallError> {
        let CallInvitation { inviter, invitee } = *invitation;
        if inviter == invitee {
            return Ok(None);
        }
        let outcome: CallOutcome = self.call_participation.call_outcome(call_id, &inviter, &invitee).await?;
        if !outcome.participated || !outcome.callee_invited_by_caller || outcome.caller_in_call || outcome.callee_pending {
            return Ok(None);
        }
        Ok(Some(CallStatus::resolve(
            outcome.callee_answered_at,
            CallStatus::conversation_end(outcome.caller_left_at, outcome.callee_left_at, ended_at),
            outcome.callee_declined,
        )))
    }

    async fn record(&self, call_id: Uuid, ended_at: DateTime<Utc>, invitation: CallInvitation, status: CallStatus) -> Result<(), LogCallError> {
        let CallInvitation { inviter, invitee } = invitation;
        let content = MessageContent::new("call".to_string())?;
        let mut tx = self.unit_of_work.begin().await?;
        tx.lock_pair(&inviter, &invitee).await?;
        match tx.consume_call_log(call_id, &inviter, &invitee).await {
            Err(RepositoryError::AlreadyExists) => return Ok(()),
            result => result?,
        };
        let mut cleared_by = Vec::new();
        for (user, peer) in [(inviter, invitee), (invitee, inviter)] {
            if CallStatus::predates(ended_at, tx.last_cleared_at(&user, &peer).await?) {
                cleared_by.push(user);
            }
        }
        if cleared_by.len() == 2 {
            return Ok(());
        }
        let recipient_read_at = tx.last_read_at(&invitee, &inviter).await?;
        let mut message = DmMessage::create_call(inviter, invitee, call_id, status, content, ended_at, recipient_read_at);
        message.hide_for(cleared_by.clone());
        let events: Vec<MessagingEvent> = message.pull_events().into_iter().map(MessagingEvent::from).collect();
        tx.insert_message(&message, &[]).await?;
        tx.hide_message(message.id(), &cleared_by).await?;
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
    use crate::ports::call_participation::EndedCall;

    #[derive(Default)]
    struct Store {
        consumed: Mutex<HashSet<(Uuid, UserId, UserId)>>,
        logged: Mutex<Vec<(UserId, UserId, String)>>,
        created: Mutex<Vec<DateTime<Utc>>>,
        cleared: Mutex<std::collections::HashMap<UserId, DateTime<Utc>>>,
        hidden: Mutex<Vec<UserId>>,
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
        async fn last_cleared_at(&mut self, user: &UserId, _: &UserId) -> Result<Option<DateTime<Utc>>, RepositoryError> {
            Ok(self.store.cleared.lock().unwrap().get(user).copied())
        }
        async fn hide_message(&mut self, _: &DmMessageId, users: &[UserId]) -> Result<(), RepositoryError> {
            self.store.hidden.lock().unwrap().extend_from_slice(users);
            Ok(())
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

    struct Outcomes {
        outcome: CallOutcome,
        ended: Mutex<Option<EndedCall>>,
        by_direction: Mutex<std::collections::HashMap<(UserId, UserId), CallOutcome>>,
    }

    #[async_trait]
    impl CallParticipationLookup for Outcomes {
        async fn call_rooms_among(&self, _: &[Uuid]) -> Result<Vec<Uuid>, CallParticipationError> {
            unimplemented!()
        }
        async fn call_outcome(&self, _: Uuid, caller: &UserId, callee: &UserId) -> Result<CallOutcome, CallParticipationError> {
            Ok(self.by_direction.lock().unwrap().get(&(*caller, *callee)).cloned().unwrap_or_else(|| self.outcome.clone()))
        }
        async fn ended_call(&self, _: Uuid) -> Result<Option<EndedCall>, CallParticipationError> {
            Ok(self.ended.lock().unwrap().clone())
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

    fn usecase(outcome: CallOutcome, ended_at: DateTime<Utc>, invitations: &[CallInvitation]) -> (Arc<Store>, LogCall<Uow, Outcomes>) {
        let (store, log_call, _) = usecase_with_lookup(outcome, ended_at, invitations);
        (store, log_call)
    }

    fn usecase_with_lookup(
        outcome: CallOutcome,
        ended_at: DateTime<Utc>,
        invitations: &[CallInvitation],
    ) -> (Arc<Store>, LogCall<Uow, Outcomes>, Arc<Outcomes>) {
        let store = Arc::new(Store::default());
        let lookup = Outcomes { outcome, ended: Mutex::new(Some(EndedCall { ended_at, invitations: invitations.to_vec() })), by_direction: Mutex::default() };
        let lookup = Arc::new(lookup);
        (store.clone(), LogCall::new(Arc::new(Uow(store)), lookup.clone()), lookup)
    }

    #[tokio::test]
    async fn an_unanswered_ended_call_leaves_one_missed_card_dated_when_the_call_ended() {
        let (caller, callee) = (UserId::new(Uuid::new_v4()), UserId::new(Uuid::new_v4()));
        let ended_at = DateTime::from_timestamp(1_800_000_000, 0).unwrap();
        let (store, log_call) = usecase(ended(false), ended_at, &[CallInvitation { inviter: caller, invitee: callee }]);
        let call_id = Uuid::new_v4();
        log_call.execute(call_id).await.unwrap();
        log_call.execute(call_id).await.unwrap();
        assert_eq!(*store.logged.lock().unwrap(), vec![(caller, callee, "missed".to_string())]);
        assert_eq!(store.created.lock().unwrap()[0], ended_at);
    }

    #[tokio::test]
    async fn a_declined_call_is_logged_as_declined_and_unverified_calls_are_skipped() {
        let (caller, callee) = (UserId::new(Uuid::new_v4()), UserId::new(Uuid::new_v4()));
        let invitation = [CallInvitation { inviter: caller, invitee: callee }];
        let (store, log_call) = usecase(ended(true), Utc::now(), &invitation);
        log_call.execute(Uuid::new_v4()).await.unwrap();
        assert_eq!(*store.logged.lock().unwrap(), vec![(caller, callee, "declined".to_string())]);

        let (store, log_call) = usecase(CallOutcome { callee_pending: true, ..ended(false) }, Utc::now(), &invitation);
        log_call.execute(Uuid::new_v4()).await.unwrap();
        assert!(store.logged.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_call_that_has_not_ended_leaves_no_card() {
        let store = Arc::new(Store::default());
        let lookup = Outcomes { outcome: ended(false), ended: Mutex::new(None), by_direction: Mutex::default() };
        LogCall::new(Arc::new(Uow(store.clone())), Arc::new(lookup)).execute(Uuid::new_v4()).await.unwrap();
        assert!(store.logged.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_clear_after_the_call_hides_the_card_only_from_whoever_cleared() {
        let ended_at = DateTime::from_timestamp(1_800_000_000, 0).unwrap();
        let after = ended_at + chrono::Duration::seconds(5);
        let (caller, callee) = (UserId::new(Uuid::new_v4()), UserId::new(Uuid::new_v4()));
        let invitation = [CallInvitation { inviter: caller, invitee: callee }];

        let (store, log_call) = usecase(ended(false), ended_at, &invitation);
        store.cleared.lock().unwrap().insert(caller, after);
        log_call.execute(Uuid::new_v4()).await.unwrap();
        assert_eq!(store.logged.lock().unwrap().len(), 1);
        assert_eq!(*store.hidden.lock().unwrap(), vec![caller]);

        let (store, log_call) = usecase(ended(false), ended_at, &invitation);
        store.cleared.lock().unwrap().extend([(caller, after), (callee, after)]);
        log_call.execute(Uuid::new_v4()).await.unwrap();
        assert!(store.logged.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn invitations_in_both_directions_leave_one_card_in_the_conversation() {
        let (a, b) = (UserId::new(Uuid::new_v4()), UserId::new(Uuid::new_v4()));
        let invitations = [CallInvitation { inviter: a, invitee: b }, CallInvitation { inviter: b, invitee: a }];
        let (store, log_call) = usecase(ended(false), Utc::now(), &invitations);
        log_call.execute(Uuid::new_v4()).await.unwrap();
        assert_eq!(*store.logged.lock().unwrap(), vec![(a, b, "missed".to_string())]);
    }

    #[tokio::test]
    async fn the_most_telling_direction_wins_the_conversation_card() {
        let (a, b) = (UserId::new(Uuid::new_v4()), UserId::new(Uuid::new_v4()));
        let invitations = [CallInvitation { inviter: a, invitee: b }, CallInvitation { inviter: b, invitee: a }];
        let ended_at = DateTime::from_timestamp(1_800_000_000, 0).unwrap();
        let (store, log_call, lookup) = usecase_with_lookup(ended(true), ended_at, &invitations);
        lookup.by_direction.lock().unwrap().insert(
            (b, a),
            CallOutcome { callee_answered_at: Some(ended_at - chrono::Duration::seconds(30)), callee_declined: false, ..ended(false) },
        );
        log_call.execute(Uuid::new_v4()).await.unwrap();
        assert_eq!(*store.logged.lock().unwrap(), vec![(b, a, "completed".to_string())]);
    }
}
