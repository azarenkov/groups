use std::sync::Arc;

use chrono::{DateTime, Utc};
use domain::{
    call::repository::CallRepository,
    channel::{events::participant::MediaSource, repository::ChannelRepository},
    shared::repository::error::RepositoryError,
    user::id::UserId,
};
use uuid::Uuid;

use crate::{
    dto::{
        context::RequestContext,
        requests::channel::{
            abort_voice_connection::AbortVoiceConnectionRequest,
            join_voice_connection::JoinVoiceConnectionRequest,
            leave_voice_connection::LeaveVoiceConnectionRequest,
            track_media_connection::TrackMediaConnectionRequest,
        },
    },
    ports::voice_webhook_verifier::{
        VoiceWebhookEvent, VoiceWebhookVerifier, VoiceWebhookVerifierError, WebhookSource,
    },
    usecases::channel::{
        abort_voice_connection::{AbortVoiceConnection, AbortVoiceConnectionError},
        join_voice_connection::{JoinVoiceConnection, JoinVoiceConnectionError},
        leave_voice_connection::{LeaveVoiceConnection, LeaveVoiceConnectionError},
        track_media_connection::{TrackMediaConnection, TrackMediaConnectionError},
    },
};

#[derive(Debug, thiserror::Error)]
pub enum HandleVoiceWebhookError {
    #[error("Webhook verifier error: {0}")]
    Verifier(#[from] VoiceWebhookVerifierError),
    #[error("Join voice connection error: {0}")]
    JoinVoiceConnection(#[from] JoinVoiceConnectionError),
    #[error("Abort voice connection error: {0}")]
    AbortVoiceConnection(#[from] AbortVoiceConnectionError),
    #[error("Leave voice connection error: {0}")]
    LeaveVoiceConnection(#[from] LeaveVoiceConnectionError),
    #[error("Track media connection error: {0}")]
    TrackMediaConnection(#[from] TrackMediaConnectionError),
    #[error("Call repository error: {0}")]
    Call(#[from] RepositoryError),
}

pub struct HandleVoiceWebhook<V, T, C>
where
    V: VoiceWebhookVerifier,
    T: ChannelRepository,
    C: CallRepository,
{
    verifier: Arc<V>,
    join_voice_connection: JoinVoiceConnection<T>,
    abort_voice_connection: AbortVoiceConnection<T>,
    leave_voice_connection: LeaveVoiceConnection<T>,
    track_media_connection: TrackMediaConnection<T>,
    call_repository: Arc<C>,
}

impl<V, T, C> HandleVoiceWebhook<V, T, C>
where
    V: VoiceWebhookVerifier,
    T: ChannelRepository,
    C: CallRepository,
{
    pub fn new(
        verifier: Arc<V>,
        join_voice_connection: JoinVoiceConnection<T>,
        abort_voice_connection: AbortVoiceConnection<T>,
        leave_voice_connection: LeaveVoiceConnection<T>,
        track_media_connection: TrackMediaConnection<T>,
        call_repository: Arc<C>,
    ) -> Self {
        Self {
            verifier,
            join_voice_connection,
            abort_voice_connection,
            leave_voice_connection,
            track_media_connection,
            call_repository,
        }
    }

    pub async fn execute(
        &self,
        body: &str,
        auth_token: &str,
        ctx: RequestContext,
    ) -> Result<(), HandleVoiceWebhookError> {
        let envelope = self.verifier.verify(body, auth_token)?;

        let source = envelope.source;
        let external_id = envelope.external_id;
        let occurred_at = envelope.occurred_at;

        match envelope.event {
            VoiceWebhookEvent::ParticipantJoined {
                channel_id,
                user_id,
                session_id,
            } => {
                if self
                    .call_repository
                    .webhook_joined(
                        channel_id,
                        &UserId::new(user_id),
                        &session_id,
                        occurred_at,
                        ctx.request_id.clone(),
                    )
                    .await?
                {
                    return Ok(());
                }
                let result = self
                    .join_voice_connection
                    .execute(JoinVoiceConnectionRequest {
                        channel_id,
                        user_id,
                        session_id,
                        occurred_at,
                        inbox_source: source,
                        inbox_external_id: external_id.clone(),
                        ctx: ctx.clone(),
                    })
                    .await;

                if let Err(e) = result {
                    if is_join_stale(&e) {
                        tracing::info!(
                            channel_id = %channel_id,
                            user_id = %user_id,
                            external_id = %external_id,
                            error = ?e,
                            "Skipping stale ParticipantJoined webhook",
                        );
                    } else {
                        return Err(e.into());
                    }
                }
            }
            VoiceWebhookEvent::ParticipantConnectionAborted {
                channel_id,
                user_id,
                session_id,
                reason,
            } => {
                if self
                    .call_repository
                    .webhook_left(
                        channel_id,
                        &UserId::new(user_id),
                        &session_id,
                        occurred_at,
                        ctx.request_id.clone(),
                    )
                    .await?
                {
                    return Ok(());
                }
                let result = self
                    .abort_voice_connection
                    .execute(AbortVoiceConnectionRequest {
                        channel_id,
                        user_id,
                        session_id,
                        reason,
                        occurred_at,
                        inbox_source: source,
                        inbox_external_id: external_id.clone(),
                        ctx: ctx.clone(),
                    })
                    .await;

                if let Err(e) = result {
                    if is_abort_stale(&e) {
                        tracing::info!(
                            channel_id = %channel_id,
                            user_id = %user_id,
                            external_id = %external_id,
                            error = ?e,
                            "Skipping stale ParticipantConnectionAborted webhook",
                        );
                    } else {
                        return Err(e.into());
                    }
                }
            }
            VoiceWebhookEvent::ParticipantLeft {
                channel_id,
                user_id,
                session_id,
                reason,
            } => {
                if self
                    .call_repository
                    .webhook_left(
                        channel_id,
                        &UserId::new(user_id),
                        &session_id,
                        occurred_at,
                        ctx.request_id.clone(),
                    )
                    .await?
                {
                    return Ok(());
                }
                let result = self
                    .leave_voice_connection
                    .execute(LeaveVoiceConnectionRequest {
                        channel_id,
                        user_id,
                        session_id,
                        reason,
                        occurred_at,
                        inbox_source: source,
                        inbox_external_id: external_id.clone(),
                        ctx: ctx.clone(),
                    })
                    .await;

                if let Err(e) = result {
                    if is_leave_stale(&e) {
                        tracing::info!(
                            channel_id = %channel_id,
                            user_id = %user_id,
                            external_id = %external_id,
                            error = ?e,
                            "Skipping stale ParticipantLeft webhook",
                        );
                    } else {
                        return Err(e.into());
                    }
                }
            }
            VoiceWebhookEvent::TrackPublished {
                channel_id,
                user_id,
                session_id,
                source: media_source,
            } => {
                self.handle_track(
                    channel_id,
                    user_id,
                    session_id,
                    media_source,
                    true,
                    occurred_at,
                    source,
                    external_id,
                    ctx,
                )
                .await?;
            }
            VoiceWebhookEvent::TrackUnpublished {
                channel_id,
                user_id,
                session_id,
                source: media_source,
            } => {
                self.handle_track(
                    channel_id,
                    user_id,
                    session_id,
                    media_source,
                    false,
                    occurred_at,
                    source,
                    external_id,
                    ctx,
                )
                .await?;
            }
            VoiceWebhookEvent::Other => {}
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn handle_track(
        &self,
        channel_id: Uuid,
        user_id: Uuid,
        session_id: String,
        source: MediaSource,
        published: bool,
        occurred_at: DateTime<Utc>,
        inbox_source: WebhookSource,
        external_id: String,
        ctx: RequestContext,
    ) -> Result<(), HandleVoiceWebhookError> {
        if self.call_repository.is_call_room(channel_id).await? {
            return Ok(());
        }

        let result = self
            .track_media_connection
            .execute(TrackMediaConnectionRequest {
                channel_id,
                user_id,
                session_id,
                source,
                published,
                occurred_at,
                inbox_source,
                inbox_external_id: external_id.clone(),
                ctx,
            })
            .await;

        if let Err(e) = result {
            let is_pre_join_race = matches!(
                &e,
                TrackMediaConnectionError::Domain(
                    domain::channel::ChannelError::ParticipantNotJoined,
                )
            );

            let give_up = if is_pre_join_race {
                Utc::now().signed_duration_since(occurred_at)
                    >= chrono::Duration::seconds(TRACK_JOIN_GRACE_SECS)
            } else {
                is_track_stale(&e)
            };

            if give_up {
                tracing::info!(
                    channel_id = %channel_id,
                    user_id = %user_id,
                    external_id = %external_id,
                    error = ?e,
                    "Skipping stale track media webhook",
                );
            } else {
                return Err(e.into());
            }
        }

        Ok(())
    }
}

const TRACK_JOIN_GRACE_SECS: i64 = 10;

fn is_track_stale(e: &TrackMediaConnectionError) -> bool {
    matches!(
        e,
        TrackMediaConnectionError::Repository(RepositoryError::NotFound)
            | TrackMediaConnectionError::Domain(_)
    )
}

fn is_join_stale(e: &JoinVoiceConnectionError) -> bool {
    matches!(
        e,
        JoinVoiceConnectionError::Repository(RepositoryError::NotFound)
            | JoinVoiceConnectionError::Domain(_)
    )
}

fn is_abort_stale(e: &AbortVoiceConnectionError) -> bool {
    matches!(
        e,
        AbortVoiceConnectionError::Repository(RepositoryError::NotFound)
            | AbortVoiceConnectionError::Domain(_)
    )
}

fn is_leave_stale(e: &LeaveVoiceConnectionError) -> bool {
    matches!(
        e,
        LeaveVoiceConnectionError::Repository(RepositoryError::NotFound)
            | LeaveVoiceConnectionError::Domain(_)
    )
}
