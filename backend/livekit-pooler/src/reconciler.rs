use std::{collections::HashMap, sync::Arc, time::Duration};

use application::{
    dto::{
        context::RequestContext,
        requests::channel::abort_voice_connection::AbortVoiceConnectionRequest,
    },
    ports::voice_webhook_verifier::WebhookSource,
    usecases::channel::abort_voice_connection::AbortVoiceConnection,
};
use chrono::Utc;
use domain::{
    channel::{events::participant::ConnectionAbortReason, repository::ChannelRepository},
    shared::repository::error::RepositoryError,
};
use livekit_api::services::{ServiceError, TwirpError, TwirpErrorCode, room::RoomClient};
use sqlx::{PgPool, Row};
use tokio::time::timeout;
use uuid::Uuid;

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

pub struct Reconciler<T>
where
    T: ChannelRepository,
{
    pool: PgPool,
    room_client_host: String,
    api_key: String,
    api_secret: String,
    abort_use_case: Arc<AbortVoiceConnection<T>>,
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
            grace,
            request_timeout,
            max_aborts_per_pass,
        }
    }

    pub async fn run_once(&self) -> Result<ReconcileSummary, ReconcileError> {
        let cutoff = Utc::now() - chrono::Duration::from_std(self.grace).unwrap_or_default();

        let rows = sqlx::query(
            r#"
            SELECT channel_id, user_id, session_id
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

        let mut by_channel: HashMap<Uuid, Vec<(Uuid, String)>> = HashMap::new();
        let mut total_entries = 0usize;
        for row in rows {
            let channel_id: Uuid = row.get(0);
            let user_id: Uuid = row.get(1);
            let session_id: String = row.get(2);
            total_entries += 1;
            by_channel
                .entry(channel_id)
                .or_default()
                .push((user_id, session_id));
        }

        let room_client = RoomClient::with_api_key(
            &self.room_client_host,
            &self.api_key,
            &self.api_secret,
        );

        let mut stale: Vec<StaleParticipant> = Vec::new();
        let mut rooms_checked = 0usize;
        let mut rooms_missing = 0usize;
        let mut livekit_errors = 0usize;

        for (channel_id, db_entries) in by_channel {
            rooms_checked += 1;

            let result = timeout(
                self.request_timeout,
                room_client.list_participants(&channel_id.to_string()),
            )
            .await;

            let live_pairs: Vec<(String, String)> = match result {
                Ok(Ok(participants)) => participants
                    .into_iter()
                    .map(|p| (p.identity, p.sid))
                    .collect(),
                Ok(Err(ServiceError::Twirp(TwirpError::Twirp(code))))
                    if code.code == TwirpErrorCode::NOT_FOUND =>
                {
                    rooms_missing += 1;
                    Vec::new()
                }
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
            };

            for (user_id, session_id) in db_entries {
                let identity = user_id.to_string();
                let still_live = live_pairs
                    .iter()
                    .any(|(id, sid)| id == &identity && sid == &session_id);
                if !still_live {
                    stale.push(StaleParticipant {
                        channel_id,
                        user_id,
                        session_id,
                    });
                    if stale.len() >= self.max_aborts_per_pass {
                        break;
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
                        application::usecases::channel::abort_voice_connection::AbortVoiceConnectionError::Repository(
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

        Ok(ReconcileSummary {
            rooms_checked,
            rooms_missing,
            livekit_errors,
            db_entries_checked: total_entries,
            stale_detected: stale.len(),
            aborted,
            abort_skipped,
            abort_failed,
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
}
