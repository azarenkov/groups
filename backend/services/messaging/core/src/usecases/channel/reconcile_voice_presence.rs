use std::{collections::HashMap, sync::Arc, time::Duration};

use chrono::Utc;
use domain::channel::webhook_source::WebhookSource;
use messaging_domain::{channel::events::participant::ConnectionAbortReason, channel::events::participant::MediaSource, channel::id::ChannelId, channel::repository::ChannelRepository, channel::repository::VoiceParticipantRow, channel::unit_of_work::ChannelUnitOfWork};
use ports::repository::RepositoryError;
use uuid::Uuid;

use crate::{
    dto::{
        context::RequestContext,
        requests::channel::{
            abort_voice_connection::AbortVoiceConnectionRequest,
            track_media_connection::TrackMediaConnectionRequest,
        },
    },
    ports::voice_room_inspector::{LiveParticipant, VoiceRoomInspector},
    usecases::channel::{
        abort_voice_connection::{AbortVoiceConnection, AbortVoiceConnectionError},
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
}

struct StaleParticipant {
    channel_id: ChannelId,
    user_id: Uuid,
    session_id: String,
}

struct TrackCorrection {
    channel_id: ChannelId,
    user_id: Uuid,
    session_id: String,
    source: MediaSource,
    enabled: bool,
}

pub struct ReconcileVoicePresence<C, W, I>
where
    C: ChannelRepository,
    W: ChannelUnitOfWork,
    I: VoiceRoomInspector,
{
    channel_repository: Arc<C>,
    inspector: Arc<I>,
    abort: Arc<AbortVoiceConnection<W>>,
    track: Arc<TrackMediaConnection<W>>,
    grace: Duration,
    max_aborts_per_pass: usize,
}

impl<C, W, I> ReconcileVoicePresence<C, W, I>
where
    C: ChannelRepository,
    W: ChannelUnitOfWork,
    I: VoiceRoomInspector,
{
    pub fn new(
        channel_repository: Arc<C>,
        inspector: Arc<I>,
        abort: Arc<AbortVoiceConnection<W>>,
        track: Arc<TrackMediaConnection<W>>,
        grace: Duration,
        max_aborts_per_pass: usize,
    ) -> Self {
        Self {
            channel_repository,
            inspector,
            abort,
            track,
            grace,
            max_aborts_per_pass,
        }
    }

    pub async fn execute(&self) -> Result<ReconcileSummary, RepositoryError> {
        let cutoff = Utc::now() - chrono::Duration::from_std(self.grace).unwrap_or_default();
        let rows = self
            .channel_repository
            .list_voice_participants_joined_before(cutoff)
            .await?;
        if rows.is_empty() {
            return Ok(ReconcileSummary::default());
        }
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

        let mut stale: Vec<StaleParticipant> = Vec::new();
        let mut corrections: Vec<TrackCorrection> = Vec::new();
        let mut rooms_checked = 0usize;
        let mut rooms_missing = 0usize;
        let mut livekit_errors = 0usize;

        for (channel_id, db_entries) in by_channel {
            rooms_checked += 1;
            let live_map: HashMap<(String, String), (bool, bool)> = if !live_rooms.contains(&channel_id) {
                rooms_missing += 1;
                HashMap::new()
            } else {
                match self.inspector.participants(&channel_id).await {
                    Ok(participants) => participants
                        .into_iter()
                        .map(|LiveParticipant { identity, session_id, camera_on, screen_on }| {
                            ((identity, session_id), (camera_on, screen_on))
                        })
                        .collect(),
                    Err(e) => {
                        livekit_errors += 1;
                        tracing::warn!(error = %e, channel_id = %channel_id.uuid(), "Failed to list voice participants; skipping channel");
                        continue;
                    }
                }
            };

            for entry in db_entries {
                let key = (entry.user_id.uuid().to_string(), entry.session_id.clone());
                match live_map.get(&key) {
                    None => {
                        stale.push(StaleParticipant {
                            channel_id,
                            user_id: entry.user_id.uuid(),
                            session_id: entry.session_id,
                        });
                        if stale.len() >= self.max_aborts_per_pass {
                            break;
                        }
                    }
                    Some(&(live_camera, live_screen)) => {
                        for (source, live, stored) in [
                            (MediaSource::Camera, live_camera, entry.camera_on),
                            (MediaSource::Screen, live_screen, entry.screen_on),
                        ] {
                            if live != stored {
                                corrections.push(TrackCorrection {
                                    channel_id,
                                    user_id: entry.user_id.uuid(),
                                    session_id: entry.session_id.clone(),
                                    source,
                                    enabled: live,
                                });
                            }
                        }
                    }
                }
            }
            if stale.len() >= self.max_aborts_per_pass {
                break;
            }
        }

        let mut aborted = 0usize;
        let mut abort_skipped = 0usize;
        let mut abort_failed = 0usize;
        for participant in &stale {
            let external_id = format!(
                "reconcile:{}:{}:{}",
                participant.channel_id.uuid(),
                participant.user_id,
                participant.session_id
            );
            let request = AbortVoiceConnectionRequest {
                channel_id: participant.channel_id.uuid(),
                user_id: participant.user_id,
                session_id: participant.session_id.clone(),
                reason: ConnectionAbortReason::StateMismatch,
                occurred_at: Utc::now(),
                inbox_source: WebhookSource::LiveKitPooler,
                inbox_external_id: external_id.clone(),
                ctx: RequestContext::default(),
            };
            match self.abort.execute(request).await {
                Ok(true) => {
                    aborted += 1;
                    tracing::info!(
                        channel_id = %participant.channel_id.uuid(),
                        user_id = %participant.user_id,
                        session_id = %participant.session_id,
                        "Reconciled stale voice participant via abort",
                    );
                }
                Ok(false) => {
                    abort_skipped += 1;
                    tracing::debug!(%external_id, "Stale participant abort was no-op (already processed or gone)");
                }
                Err(AbortVoiceConnectionError::Repository(RepositoryError::NotFound)) => {
                    abort_failed += 1;
                    tracing::debug!(%external_id, "Stale participant already removed before abort applied");
                }
                Err(e) => {
                    abort_failed += 1;
                    tracing::warn!(error = ?e, %external_id, "Failed to apply stale participant abort");
                }
            }
        }

        let mut tracks_reconciled = 0usize;
        for correction in &corrections {
            let source_str = match correction.source {
                MediaSource::Camera => "camera",
                MediaSource::Screen => "screen",
            };
            let external_id = format!(
                "reconcile-track:{}:{}:{}:{}:{}:{}",
                correction.channel_id.uuid(),
                correction.user_id,
                correction.session_id,
                source_str,
                correction.enabled,
                Utc::now().timestamp_millis(),
            );
            let request = TrackMediaConnectionRequest {
                channel_id: correction.channel_id.uuid(),
                user_id: correction.user_id,
                session_id: correction.session_id.clone(),
                source: correction.source,
                published: correction.enabled,
                occurred_at: Utc::now(),
                inbox_source: WebhookSource::LiveKitPooler,
                inbox_external_id: external_id.clone(),
                ctx: RequestContext::default(),
            };
            match self.track.execute(request).await {
                Ok(_) => {
                    tracks_reconciled += 1;
                    tracing::info!(
                        channel_id = %correction.channel_id.uuid(),
                        user_id = %correction.user_id,
                        session_id = %correction.session_id,
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

        Ok(ReconcileSummary {
            rooms_checked,
            rooms_missing,
            livekit_errors,
            db_entries_checked: total_entries,
            stale_detected: stale.len(),
            aborted,
            abort_skipped,
            abort_failed,
            tracks_reconciled,
        })
    }
}
