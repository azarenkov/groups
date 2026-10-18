use std::{collections::HashMap, collections::HashSet, sync::Arc};

use chrono::{DateTime, Duration, Utc};
use domain::{channel::id::ChannelId, user::id::UserId};
use ports::repository::RepositoryError;
use uuid::Uuid;
use voice_domain::call::repository::{CallRepository, ConnectedSession};

use crate::ports::voice_service::VoiceService;

pub const SESSION_PROBE_GRACE_SECS: i64 = 20;

#[derive(Debug, thiserror::Error)]
pub enum ReconcileCallSessionsError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ReconcileCallSummary {
    pub calls_checked: usize,
    pub sessions_checked: usize,
    pub dropped: usize,
    pub livekit_errors: usize,
}

pub struct ReconcileCallSessions<C, V>
where
    C: CallRepository,
    V: VoiceService,
{
    call_repository: Arc<C>,
    voice_service: Arc<V>,
}

impl<C, V> ReconcileCallSessions<C, V>
where
    C: CallRepository,
    V: VoiceService,
{
    pub fn new(call_repository: Arc<C>, voice_service: Arc<V>) -> Self {
        Self { call_repository, voice_service }
    }

    pub async fn execute(&self, now: DateTime<Utc>) -> Result<ReconcileCallSummary, ReconcileCallSessionsError> {
        let settled_before = now - Duration::seconds(SESSION_PROBE_GRACE_SECS);
        let sessions = self.call_repository.list_connected_sessions(settled_before).await?;
        let mut summary = ReconcileCallSummary { sessions_checked: sessions.len(), ..ReconcileCallSummary::default() };
        let mut by_call: HashMap<Uuid, Vec<ConnectedSession>> = HashMap::new();
        for session in sessions {
            by_call.entry(session.call_id).or_default().push(session);
        }

        for (call_id, sessions) in by_call {
            summary.calls_checked += 1;
            let present: HashSet<UserId> = match self.voice_service.room_participants(&ChannelId::new(call_id)).await {
                Ok(users) => users.into_iter().collect(),
                Err(e) => {
                    summary.livekit_errors += 1;
                    tracing::warn!(error = %e, call_id = %call_id, "Failed to list call room participants; skipping call");
                    continue;
                }
            };
            for session in sessions.iter().filter(|s| !present.contains(&s.user_id)) {
                self.call_repository
                    .webhook_dropped(call_id, &session.user_id, &session.session_id, now)
                    .await?;
                summary.dropped += 1;
                tracing::info!(
                    call_id = %call_id,
                    user_id = %session.user_id.uuid(),
                    session_id = %session.session_id,
                    "Call participant is gone from LiveKit; started the reconnect grace",
                );
            }
        }
        Ok(summary)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use ports::fakes::FakeVoiceService;
    use voice_domain::call::repository::{
        AddParticipantsOutcome, CallPairOutcome, CallParticipantView, EndedCall, JoinOutcome, PendingEviction, WebhookJoinOutcome,
    };

    use super::*;

    #[derive(Default)]
    struct Calls {
        connected: Vec<ConnectedSession>,
        dropped: Mutex<Vec<(Uuid, UserId, String)>>,
    }

    #[async_trait]
    impl CallRepository for Calls {
        async fn start_call(&self, _: Uuid, _: &UserId, _: &[UserId], _: bool, _: DateTime<Utc>, _: Option<String>) -> Result<(), RepositoryError> {
            unimplemented!()
        }
        async fn call_video(&self, _: Uuid) -> Result<bool, RepositoryError> {
            unimplemented!()
        }
        async fn respond(&self, _: Uuid, _: &UserId, _: bool, _: DateTime<Utc>, _: Option<String>) -> Result<bool, RepositoryError> {
            unimplemented!()
        }
        async fn add_participants(&self, _: Uuid, _: &UserId, _: &[UserId], _: usize, _: DateTime<Utc>, _: Option<String>) -> Result<AddParticipantsOutcome, RepositoryError> {
            unimplemented!()
        }
        async fn leave(&self, _: Uuid, _: &UserId, _: DateTime<Utc>, _: Option<String>) -> Result<(), RepositoryError> {
            unimplemented!()
        }
        async fn leave_many(&self, _: Uuid, _: &[UserId], _: DateTime<Utc>, _: Option<String>) -> Result<(), RepositoryError> {
            unimplemented!()
        }
        async fn evict_many(&self, _: Uuid, _: &[UserId], _: DateTime<Utc>, _: Option<String>) -> Result<(), RepositoryError> {
            unimplemented!()
        }
        async fn claim_due_evictions(&self, _: DateTime<Utc>, _: i64) -> Result<Vec<PendingEviction>, RepositoryError> {
            unimplemented!()
        }
        async fn eviction_done(&self, _: Uuid, _: &UserId) -> Result<(), RepositoryError> {
            unimplemented!()
        }
        async fn eviction_failed(&self, _: Uuid, _: &UserId, _: DateTime<Utc>) -> Result<(), RepositoryError> {
            unimplemented!()
        }
        async fn is_active_participant(&self, _: Uuid, _: &UserId) -> Result<bool, RepositoryError> {
            unimplemented!()
        }
        async fn is_joined_participant(&self, _: Uuid, _: &UserId) -> Result<bool, RepositoryError> {
            unimplemented!()
        }
        async fn list_active(&self, _: Uuid) -> Result<Vec<CallParticipantView>, RepositoryError> {
            unimplemented!()
        }
        async fn active_participant_ids(&self, _: Uuid) -> Result<Vec<UserId>, RepositoryError> {
            unimplemented!()
        }
        async fn active_call_for(&self, _: &UserId) -> Result<Option<Uuid>, RepositoryError> {
            unimplemented!()
        }
        async fn joined_at(&self, _: Uuid, _: &UserId) -> Result<Option<DateTime<Utc>>, RepositoryError> {
            unimplemented!()
        }
        async fn active_call_between(&self, _: &UserId, _: &UserId) -> Result<Option<Uuid>, RepositoryError> {
            unimplemented!()
        }
        async fn shared_active_calls(&self, _: &UserId, _: &UserId) -> Result<Vec<Uuid>, RepositoryError> {
            unimplemented!()
        }
        async fn join(&self, _: Uuid, _: &UserId, _: usize, _: DateTime<Utc>) -> Result<JoinOutcome, RepositoryError> {
            unimplemented!()
        }
        async fn is_call_room(&self, _: Uuid) -> Result<bool, RepositoryError> {
            unimplemented!()
        }
        async fn call_rooms_among(&self, _: &[Uuid]) -> Result<Vec<Uuid>, RepositoryError> {
            unimplemented!()
        }
        async fn pair_outcome(&self, _: Uuid, _: &UserId, _: &UserId) -> Result<CallPairOutcome, RepositoryError> {
            unimplemented!()
        }
        async fn ended_call(&self, _: Uuid) -> Result<Option<EndedCall>, RepositoryError> {
            unimplemented!()
        }
        async fn webhook_joined(&self, _: Uuid, _: &UserId, _: &str, _: usize, _: DateTime<Utc>, _: Option<String>) -> Result<WebhookJoinOutcome, RepositoryError> {
            unimplemented!()
        }
        async fn webhook_left(&self, _: Uuid, _: &UserId, _: &str, _: DateTime<Utc>, _: Option<String>) -> Result<bool, RepositoryError> {
            unimplemented!()
        }
        async fn webhook_dropped(&self, call_id: Uuid, user_id: &UserId, session_id: &str, _: DateTime<Utc>) -> Result<bool, RepositoryError> {
            self.dropped.lock().unwrap().push((call_id, *user_id, session_id.to_string()));
            Ok(true)
        }
        async fn expire_reconnects(&self, _: DateTime<Utc>, _: i64, _: Option<String>) -> Result<usize, RepositoryError> {
            unimplemented!()
        }
        async fn list_connected_sessions(&self, _: DateTime<Utc>) -> Result<Vec<ConnectedSession>, RepositoryError> {
            Ok(self.connected.clone())
        }
        async fn expire_invitations(&self, _: DateTime<Utc>, _: i64, _: Option<String>) -> Result<usize, RepositoryError> {
            unimplemented!()
        }
    }

    fn session(call_id: Uuid, user_id: UserId, session_id: &str) -> ConnectedSession {
        ConnectedSession { call_id, user_id, session_id: session_id.to_string() }
    }

    #[tokio::test]
    async fn drops_recorded_sessions_that_livekit_no_longer_lists() {
        let (call, other_call) = (Uuid::new_v4(), Uuid::new_v4());
        let (present, gone, alone) = (UserId::generate(), UserId::generate(), UserId::generate());
        let calls = Arc::new(Calls {
            connected: vec![session(call, present, "s1"), session(call, gone, "s2"), session(other_call, alone, "s3")],
            ..Calls::default()
        });
        let voice = Arc::new(FakeVoiceService::default().room(ChannelId::new(call), vec![present]));
        let summary = ReconcileCallSessions::new(calls.clone(), voice).execute(Utc::now()).await.unwrap();
        assert_eq!(summary, ReconcileCallSummary { calls_checked: 2, sessions_checked: 3, dropped: 2, livekit_errors: 0 });
        let mut dropped = calls.dropped.lock().unwrap().clone();
        dropped.sort_by_key(|(call, _, _)| *call == other_call);
        assert_eq!(dropped, vec![(call, gone, "s2".to_string()), (other_call, alone, "s3".to_string())]);
    }

    #[tokio::test]
    async fn leaves_sessions_alone_when_livekit_cannot_be_asked() {
        let call = Uuid::new_v4();
        let calls = Arc::new(Calls { connected: vec![session(call, UserId::generate(), "s1")], ..Calls::default() });
        let summary = ReconcileCallSessions::new(calls.clone(), Arc::new(FakeVoiceService::unreachable())).execute(Utc::now()).await.unwrap();
        assert_eq!(summary.dropped, 0);
        assert!(calls.dropped.lock().unwrap().is_empty());
    }
}
