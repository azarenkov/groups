use std::{collections::HashMap, collections::HashSet, sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use domain::channel::webhook_source::WebhookSource;
use domain::user::id::UserId;
use messaging_domain::{channel::events::participant::ConnectionAbortReason, channel::events::participant::MediaSource, channel::id::ChannelId, channel::repository::ChannelRepository, channel::repository::VoiceParticipantRow, channel::unit_of_work::ChannelUnitOfWork, community::repository::CommunityRepository};
use ports::repository::RepositoryError;
use ports::voice_service::VoiceService;
use uuid::Uuid;

use crate::{
    dto::{
        context::RequestContext,
        requests::channel::{
            abort_voice_connection::AbortVoiceConnectionRequest,
            join_voice_connection::JoinVoiceConnectionRequest,
            track_media_connection::TrackMediaConnectionRequest,
        },
    },
    ports::voice_room_inspector::{LiveParticipant, VoiceRoomInspector},
    usecases::channel::{
        abort_voice_connection::{AbortVoiceConnection, AbortVoiceConnectionError},
        admit_voice_participant::{AdmitVoiceParticipant, Admission},
        join_voice_connection::{JoinVoiceConnection, JoinVoiceConnectionError},
        track_media_connection::TrackMediaConnection,
    },
};

#[derive(Debug, Default, Clone, Copy)]
pub struct ReconcileSummary {
    pub rooms_checked: usize,
    pub rooms_missing: usize,
    pub livekit_errors: usize,
    pub db_entries_checked: usize,
    pub stale_detected: usize,
    pub aborted: usize,
    pub abort_skipped: usize,
    pub abort_failed: usize,
    pub tracks_reconciled: usize,
    pub access_reapplied: usize,
    pub access_failed: usize,
    pub rejected: usize,
    pub joins_restored: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Session {
    channel_id: ChannelId,
    user_id: Uuid,
    session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrackCorrection {
    session: Session,
    source: MediaSource,
    enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LiveMember {
    session: Session,
    joined_at: Option<DateTime<Utc>>,
    recorded: bool,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct RoomPlan {
    stale: Vec<Session>,
    corrections: Vec<TrackCorrection>,
    live: Vec<LiveMember>,
}

fn plan_room(
    channel_id: ChannelId,
    db_entries: &[VoiceParticipantRow],
    live: Option<&[LiveParticipant]>,
    settled_before: DateTime<Utc>,
) -> RoomPlan {
    let mut plan = RoomPlan::default();
    let live_map: HashMap<(Uuid, &str), &LiveParticipant> = live
        .unwrap_or_default()
        .iter()
        .filter_map(|p| Uuid::parse_str(&p.identity).ok().map(|user| ((user, p.session_id.as_str()), p)))
        .collect();
    let recorded: HashSet<(Uuid, &str)> =
        db_entries.iter().map(|e| (e.user_id.uuid(), e.session_id.as_str())).collect();

    for entry in db_entries {
        let session = Session { channel_id, user_id: entry.user_id.uuid(), session_id: entry.session_id.clone() };
        match live_map.get(&(entry.user_id.uuid(), entry.session_id.as_str())) {
            None => plan.stale.push(session),
            Some(participant) => {
                for (source, live, stored) in [
                    (MediaSource::Camera, participant.camera_on, entry.camera_on),
                    (MediaSource::Screen, participant.screen_on, entry.screen_on),
                ] {
                    if live != stored {
                        plan.corrections.push(TrackCorrection { session: session.clone(), source, enabled: live });
                    }
                }
            }
        }
    }

    for ((user_id, session_id), participant) in live_map {
        if participant.joined_at.is_some_and(|at| at > settled_before) {
            continue;
        }
        plan.live.push(LiveMember {
            session: Session { channel_id, user_id, session_id: session_id.to_string() },
            joined_at: participant.joined_at,
            recorded: recorded.contains(&(user_id, session_id)),
        });
    }
    plan.live.sort_by(|a, b| (a.session.user_id, &a.session.session_id).cmp(&(b.session.user_id, &b.session.session_id)));
    plan
}

pub struct ReconcileActions<C, S, W, V>
where
    C: ChannelRepository,
    S: CommunityRepository,
    W: ChannelUnitOfWork,
    V: VoiceService,
{
    pub admit: Arc<AdmitVoiceParticipant<C, S, V>>,
    pub join: Arc<JoinVoiceConnection<W>>,
    pub abort: Arc<AbortVoiceConnection<W>>,
    pub track: Arc<TrackMediaConnection<W>>,
}

#[derive(Debug, Clone, Copy)]
pub struct ReconcileLimits {
    pub grace: Duration,
    pub max_aborts_per_pass: usize,
}

pub struct ReconcileVoicePresence<C, S, W, V, I>
where
    C: ChannelRepository,
    S: CommunityRepository,
    W: ChannelUnitOfWork,
    V: VoiceService,
    I: VoiceRoomInspector,
{
    channel_repository: Arc<C>,
    inspector: Arc<I>,
    actions: ReconcileActions<C, S, W, V>,
    limits: ReconcileLimits,
}

impl<C, S, W, V, I> ReconcileVoicePresence<C, S, W, V, I>
where
    C: ChannelRepository,
    S: CommunityRepository,
    W: ChannelUnitOfWork,
    V: VoiceService,
    I: VoiceRoomInspector,
{
    pub fn new(channel_repository: Arc<C>, inspector: Arc<I>, actions: ReconcileActions<C, S, W, V>, limits: ReconcileLimits) -> Self {
        Self { channel_repository, inspector, actions, limits }
    }

    pub async fn execute(&self) -> Result<ReconcileSummary, RepositoryError> {
        self.execute_with_checkpoint(&|| true).await
    }

    pub async fn execute_with_checkpoint(
        &self,
        checkpoint: &(dyn Fn() -> bool + Sync),
    ) -> Result<ReconcileSummary, RepositoryError> {
        let cutoff = Utc::now() - chrono::Duration::from_std(self.limits.grace).unwrap_or_default();
        let rows = self
            .channel_repository
            .list_voice_participants_joined_before(cutoff)
            .await?;
        let total_entries = rows.len();
        let mut by_channel: HashMap<ChannelId, Vec<VoiceParticipantRow>> = HashMap::new();
        for row in rows {
            by_channel.entry(row.channel_id).or_default().push(row);
        }

        let live_rooms = match self.inspector.live_rooms().await {
            Ok(rooms) => rooms,
            Err(e) => {
                tracing::warn!(error = %e, "Voice room probe failed; skipping reconciliation pass");
                return Ok(ReconcileSummary {
                    livekit_errors: 1,
                    db_entries_checked: total_entries,
                    ..ReconcileSummary::default()
                });
            }
        };
        if by_channel.is_empty() && live_rooms.is_empty() {
            return Ok(ReconcileSummary::default());
        }

        let mut summary = ReconcileSummary { db_entries_checked: total_entries, ..ReconcileSummary::default() };
        let mut stale: Vec<Session> = Vec::new();
        let mut corrections: Vec<TrackCorrection> = Vec::new();
        let mut live: Vec<LiveMember> = Vec::new();

        let channels: HashSet<ChannelId> = by_channel.keys().copied().chain(live_rooms.iter().copied()).collect();
        for channel_id in channels {
            if !checkpoint() {
                break;
            }
            summary.rooms_checked += 1;
            let participants = if !live_rooms.contains(&channel_id) {
                summary.rooms_missing += 1;
                None
            } else {
                match self.inspector.participants(&channel_id).await {
                    Ok(participants) => Some(participants),
                    Err(e) => {
                        summary.livekit_errors += 1;
                        tracing::warn!(error = %e, channel_id = %channel_id.uuid(), "Failed to list voice participants; skipping channel");
                        continue;
                    }
                }
            };
            let db_entries = by_channel.remove(&channel_id).unwrap_or_default();
            let plan = plan_room(channel_id, &db_entries, participants.as_deref(), cutoff);
            let room = self.limits.max_aborts_per_pass.saturating_sub(stale.len());
            stale.extend(plan.stale.into_iter().take(room));
            corrections.extend(plan.corrections);
            live.extend(plan.live);
        }

        summary.stale_detected = stale.len();
        for session in &stale {
            if !checkpoint() {
                break;
            }
            self.abort_stale(session, &mut summary).await;
        }

        for correction in &corrections {
            if !checkpoint() {
                break;
            }
            self.correct_track(correction, &mut summary).await;
        }

        for member in &live {
            if !checkpoint() {
                break;
            }
            self.enforce_member(member, &mut summary).await;
        }

        Ok(summary)
    }

    async fn abort_stale(&self, session: &Session, summary: &mut ReconcileSummary) {
        let external_id = format!("reconcile:{}:{}:{}", session.channel_id.uuid(), session.user_id, session.session_id);
        let request = AbortVoiceConnectionRequest {
            channel_id: session.channel_id.uuid(),
            user_id: session.user_id,
            session_id: session.session_id.clone(),
            reason: ConnectionAbortReason::StateMismatch,
            occurred_at: Utc::now(),
            inbox_source: WebhookSource::LiveKitPooler,
            inbox_external_id: external_id.clone(),
            ctx: RequestContext::default(),
        };
        match self.actions.abort.execute(request).await {
            Ok(true) => {
                summary.aborted += 1;
                tracing::info!(
                    channel_id = %session.channel_id.uuid(),
                    user_id = %session.user_id,
                    session_id = %session.session_id,
                    "Reconciled stale voice participant via abort",
                );
            }
            Ok(false) => {
                summary.abort_skipped += 1;
                tracing::debug!(%external_id, "Stale participant abort was no-op (already processed or gone)");
            }
            Err(AbortVoiceConnectionError::Repository(RepositoryError::NotFound)) => {
                summary.abort_failed += 1;
                tracing::debug!(%external_id, "Stale participant already removed before abort applied");
            }
            Err(e) => {
                summary.abort_failed += 1;
                tracing::warn!(error = ?e, %external_id, "Failed to apply stale participant abort");
            }
        }
    }

    async fn correct_track(&self, correction: &TrackCorrection, summary: &mut ReconcileSummary) {
        let session = &correction.session;
        let source_str = match correction.source {
            MediaSource::Camera => "camera",
            MediaSource::Screen => "screen",
        };
        let external_id = format!(
            "reconcile-track:{}:{}:{}:{}:{}:{}",
            session.channel_id.uuid(),
            session.user_id,
            session.session_id,
            source_str,
            correction.enabled,
            Utc::now().timestamp_millis(),
        );
        let request = TrackMediaConnectionRequest {
            channel_id: session.channel_id.uuid(),
            user_id: session.user_id,
            session_id: session.session_id.clone(),
            source: correction.source,
            published: correction.enabled,
            occurred_at: Utc::now(),
            inbox_source: WebhookSource::LiveKitPooler,
            inbox_external_id: external_id.clone(),
            ctx: RequestContext::default(),
        };
        match self.actions.track.execute(request).await {
            Ok(_) => {
                summary.tracks_reconciled += 1;
                tracing::info!(
                    channel_id = %session.channel_id.uuid(),
                    user_id = %session.user_id,
                    session_id = %session.session_id,
                    source = source_str,
                    enabled = correction.enabled,
                    "Reconciled drifted track media state",
                );
            }
            Err(e) => {
                tracing::warn!(error = ?e, %external_id, "Failed to reconcile track media state");
            }
        }
    }

    async fn enforce_member(&self, member: &LiveMember, summary: &mut ReconcileSummary) {
        let session = &member.session;
        match self.actions.admit.execute(session.channel_id, UserId::new(session.user_id)).await {
            Ok(Admission::Admitted) => summary.access_reapplied += 1,
            Ok(Admission::Rejected) => {
                summary.rejected += 1;
                return;
            }
            Err(e) => {
                summary.access_failed += 1;
                tracing::warn!(error = ?e, channel_id = %session.channel_id.uuid(), user_id = %session.user_id, "Failed to reapply voice participant access");
                return;
            }
        }
        if member.recorded {
            return;
        }
        let external_id = format!("reconcile-join:{}:{}:{}", session.channel_id.uuid(), session.user_id, session.session_id);
        let request = JoinVoiceConnectionRequest {
            channel_id: session.channel_id.uuid(),
            user_id: session.user_id,
            session_id: session.session_id.clone(),
            occurred_at: member.joined_at.unwrap_or_else(Utc::now),
            inbox_source: WebhookSource::LiveKitPooler,
            inbox_external_id: external_id.clone(),
            ctx: RequestContext::default(),
        };
        match self.actions.join.execute(request).await {
            Ok(true) => {
                summary.joins_restored += 1;
                tracing::info!(
                    channel_id = %session.channel_id.uuid(),
                    user_id = %session.user_id,
                    session_id = %session.session_id,
                    "Restored voice participant missing from the channel state",
                );
            }
            Ok(false) => tracing::debug!(%external_id, "Missing participant was already restored"),
            Err(JoinVoiceConnectionError::Repository(RepositoryError::NotFound)) => {
                tracing::debug!(%external_id, "Channel vanished before the missing participant was restored");
            }
            Err(e) => tracing::warn!(error = ?e, %external_id, "Failed to restore missing voice participant"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(channel_id: ChannelId, user: Uuid, session: &str, camera_on: bool, screen_on: bool) -> VoiceParticipantRow {
        VoiceParticipantRow { channel_id, user_id: UserId::new(user), session_id: session.to_string(), camera_on, screen_on }
    }

    fn live(user: Uuid, session: &str, camera_on: bool, joined_at: Option<DateTime<Utc>>) -> LiveParticipant {
        LiveParticipant { identity: user.to_string(), session_id: session.to_string(), camera_on, screen_on: false, joined_at }
    }

    #[test]
    fn a_room_that_is_not_live_marks_every_recorded_session_stale() {
        let channel = ChannelId::generate();
        let user = Uuid::new_v4();
        let plan = plan_room(channel, &[row(channel, user, "s1", false, false)], None, Utc::now());
        assert_eq!(plan.stale, vec![Session { channel_id: channel, user_id: user, session_id: "s1".into() }]);
        assert!(plan.live.is_empty());
    }

    #[test]
    fn live_participants_are_enforced_and_unrecorded_ones_restored() {
        let channel = ChannelId::generate();
        let (known, unknown) = (Uuid::new_v4(), Uuid::new_v4());
        let settled = Utc::now() - chrono::Duration::seconds(20);
        let participants = vec![
            live(known, "s1", true, Some(settled - chrono::Duration::minutes(1))),
            live(unknown, "s2", false, Some(settled - chrono::Duration::minutes(1))),
        ];
        let plan = plan_room(channel, &[row(channel, known, "s1", false, false)], Some(&participants), settled);
        assert!(plan.stale.is_empty());
        assert_eq!(plan.corrections.len(), 1);
        assert_eq!((plan.corrections[0].source, plan.corrections[0].enabled), (MediaSource::Camera, true));
        let mut by_user: Vec<(Uuid, bool)> = plan.live.iter().map(|m| (m.session.user_id, m.recorded)).collect();
        by_user.sort();
        let mut expected = vec![(known, true), (unknown, false)];
        expected.sort();
        assert_eq!(by_user, expected);
    }

    #[test]
    fn participants_still_inside_the_grace_window_are_left_alone() {
        let channel = ChannelId::generate();
        let fresh = Uuid::new_v4();
        let settled = Utc::now() - chrono::Duration::seconds(20);
        let participants = vec![live(fresh, "s1", false, Some(Utc::now())), LiveParticipant {
            identity: "not-a-user".into(),
            session_id: "s9".into(),
            camera_on: false,
            screen_on: false,
            joined_at: None,
        }];
        let plan = plan_room(channel, &[], Some(&participants), settled);
        assert!(plan.live.is_empty());
    }
}
