use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use contracts::events::webhook::WebhookSource;
use messaging::{
    dto::{
        context::RequestContext,
        requests::channel::{
            abort_voice_connection::AbortVoiceConnectionRequest,
            track_media_connection::TrackMediaConnectionRequest,
        },
    },
    usecases::channel::{
        abort_voice_connection::AbortVoiceConnection,
        track_media_connection::TrackMediaConnection,
    },
};
use chrono::Utc;
use messaging_domain::{channel::events::participant::ConnectionAbortReason, channel::events::participant::MediaSource, channel::repository::ChannelRepository};
use ports::repository::RepositoryError;
use livekit_api::services::room::RoomClient;
use sqlx::{PgPool, Row};
use tokio::time::timeout;
use uuid::Uuid;

const LK_SOURCE_CAMERA: i32 = 1;
const LK_SOURCE_SCREEN: i32 = 3;

#[derive(Debug, thiserror::Error)]
pub enum ReconcileError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

#[derive(Debug, Clone)]
struct StaleParticipant {
    channel_id: Uuid,
    user_id: Uuid,
    session_id: String,
}

#[derive(Debug, Clone)]
struct TrackCorrection {
    channel_id: Uuid,
    user_id: Uuid,
    session_id: String,
    source: MediaSource,
    enabled: bool,
}

pub struct Reconciler<T>
where
    T: ChannelRepository,
{
    pool: PgPool,
    room_client_host: String,
    api_key: String,
    api_secret: String,
    abort_use_case: Arc<AbortVoiceConnection<T>>,
    track_use_case: Arc<TrackMediaConnection<T>>,
    grace: Duration,
    request_timeout: Duration,
    max_aborts_per_pass: usize,
}

impl<T> Reconciler<T>
where
    T: ChannelRepository,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pool: PgPool,
        room_client_host: String,
        api_key: String,
        api_secret: String,
        abort_use_case: Arc<AbortVoiceConnection<T>>,
        track_use_case: Arc<TrackMediaConnection<T>>,
        grace: Duration,
        request_timeout: Duration,
        max_aborts_per_pass: usize,
    ) -> Self {
        Self {
            pool,
            room_client_host,
            api_key,
            api_secret,
            abort_use_case,
            track_use_case,
            grace,
            request_timeout,
            max_aborts_per_pass,
        }
    }

    pub async fn run_once(&self) -> Result<ReconcileSummary, ReconcileError> {
        let cutoff = Utc::now() - chrono::Duration::from_std(self.grace).unwrap_or_default();

        let rows = sqlx::query(
            r#"
            SELECT channel_id, user_id, session_id, camera_on, screen_on
            FROM channel_voice_participants
            WHERE joined_at < $1
            "#,
        )
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await?;

        if rows.is_empty() {
            return Ok(ReconcileSummary::default());
        }

        let mut by_channel: HashMap<Uuid, Vec<(Uuid, String, bool, bool)>> = HashMap::new();
        let mut total_entries = 0usize;
        for row in rows {
            let channel_id: Uuid = row.get(0);
            let user_id: Uuid = row.get(1);
            let session_id: String = row.get(2);
            let camera_on: bool = row.get(3);
            let screen_on: bool = row.get(4);
            total_entries += 1;
            by_channel
                .entry(channel_id)
                .or_default()
                .push((user_id, session_id, camera_on, screen_on));
        }

        let room_client = RoomClient::with_api_key(
            &self.room_client_host,
            &self.api_key,
            &self.api_secret,
        );

        let probe = timeout(
            self.request_timeout,
            room_client.list_rooms(Vec::new()),
        )
        .await;

        let live_room_names: HashSet<String> = match probe {
            Ok(Ok(rooms)) => rooms.into_iter().map(|r| r.name).collect(),
            Ok(Err(e)) => {
                tracing::warn!(
                    error = %e,
                    "LiveKit list_rooms probe failed; skipping reconciliation pass",
                );
                return Ok(ReconcileSummary {
                    rooms_checked: 0,
                    rooms_missing: 0,
                    livekit_errors: 1,
                    db_entries_checked: total_entries,
                    stale_detected: 0,
                    aborted: 0,
                    abort_skipped: 0,
                    abort_failed: 0,
                    tracks_reconciled: 0,
                });
            }
            Err(_) => {
                tracing::warn!(
                    timeout_secs = self.request_timeout.as_secs(),
                    "LiveKit list_rooms probe timed out; skipping reconciliation pass",
                );
                return Ok(ReconcileSummary {
                    rooms_checked: 0,
                    rooms_missing: 0,
                    livekit_errors: 1,
                    db_entries_checked: total_entries,
                    stale_detected: 0,
                    aborted: 0,
                    abort_skipped: 0,
                    abort_failed: 0,
                    tracks_reconciled: 0,
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

            let live_map: HashMap<(String, String), (bool, bool)> = if !live_room_names
                .contains(&channel_id.to_string())
            {
                rooms_missing += 1;
                HashMap::new()
            } else {
                let result = timeout(
                    self.request_timeout,
                    room_client.list_participants(&channel_id.to_string()),
                )
                .await;
                match result {
                    Ok(Ok(participants)) => participants
                        .into_iter()
                        .map(|p| {
                            let camera = p
                                .tracks
                                .iter()
                                .any(|t| t.source == LK_SOURCE_CAMERA);
                            let screen = p
                                .tracks
                                .iter()
                                .any(|t| t.source == LK_SOURCE_SCREEN);
                            ((p.identity, p.sid), (camera, screen))
                        })
                        .collect(),
                    Ok(Err(e)) => {
                        livekit_errors += 1;
                        tracing::warn!(
                            error = %e,
                            %channel_id,
                            "Failed to list LiveKit participants; skipping channel",
                        );
                        continue;
                    }
                    Err(_) => {
                        livekit_errors += 1;
                        tracing::warn!(
                            %channel_id,
                            timeout_secs = self.request_timeout.as_secs(),
                            "LiveKit list_participants timed out; skipping channel",
                        );
                        continue;
                    }
                }
            };

            for (user_id, session_id, camera_on, screen_on) in db_entries {
                let identity = user_id.to_string();
                match live_map.get(&(identity, session_id.clone())) {
                    None => {
                        stale.push(StaleParticipant {
                            channel_id,
                            user_id,
                            session_id,
                        });
                        if stale.len() >= self.max_aborts_per_pass {
                            break;
                        }
                    }
                    Some(&(live_camera, live_screen)) => {
                        if live_camera != camera_on {
                            corrections.push(TrackCorrection {
                                channel_id,
                                user_id,
                                session_id: session_id.clone(),
                                source: MediaSource::Camera,
                                enabled: live_camera,
                            });
                        }
                        if live_screen != screen_on {
                            corrections.push(TrackCorrection {
                                channel_id,
                                user_id,
                                session_id: session_id.clone(),
                                source: MediaSource::Screen,
                                enabled: live_screen,
                            });
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
                participant.channel_id, participant.user_id, participant.session_id
            );
            let request = AbortVoiceConnectionRequest {
                channel_id: participant.channel_id,
                user_id: participant.user_id,
                session_id: participant.session_id.clone(),
                reason: ConnectionAbortReason::StateMismatch,
                occurred_at: Utc::now(),
                inbox_source: WebhookSource::LiveKitPooler,
                inbox_external_id: external_id.clone(),
                ctx: RequestContext::default(),
            };

            match self.abort_use_case.execute(request).await {
                Ok(true) => {
                    aborted += 1;
                    tracing::info!(
                        channel_id = %participant.channel_id,
                        user_id = %participant.user_id,
                        session_id = %participant.session_id,
                        "Reconciled stale voice participant via abort",
                    );
                }
                Ok(false) => {
                    abort_skipped += 1;
                    tracing::debug!(
                        %external_id,
                        "Stale participant abort was no-op (already processed or gone)",
                    );
                }
                Err(e) => {
                    abort_failed += 1;
                    let stale_not_found = matches!(
                        &e,
                        messaging::usecases::channel::abort_voice_connection::AbortVoiceConnectionError::Repository(
                            RepositoryError::NotFound,
                        ),
                    );
                    if stale_not_found {
                        tracing::debug!(
                            %external_id,
                            "Stale participant already removed before abort applied",
                        );
                    } else {
                        tracing::warn!(
                            error = ?e,
                            %external_id,
                            "Failed to apply stale participant abort",
                        );
                    }
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
                correction.channel_id,
                correction.user_id,
                correction.session_id,
                source_str,
                correction.enabled,
                Utc::now().timestamp_millis(),
            );
            let request = TrackMediaConnectionRequest {
                channel_id: correction.channel_id,
                user_id: correction.user_id,
                session_id: correction.session_id.clone(),
                source: correction.source,
                published: correction.enabled,
                occurred_at: Utc::now(),
                inbox_source: WebhookSource::LiveKitPooler,
                inbox_external_id: external_id.clone(),
                ctx: RequestContext::default(),
            };

            match self.track_use_case.execute(request).await {
                Ok(_) => {
                    tracks_reconciled += 1;
                    tracing::info!(
                        channel_id = %correction.channel_id,
                        user_id = %correction.user_id,
                        session_id = %correction.session_id,
                        source = source_str,
                        enabled = correction.enabled,
                        "Reconciled drifted track media state",
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        error = ?e,
                        %external_id,
                        "Failed to reconcile track media state",
                    );
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
