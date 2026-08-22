use std::sync::Arc;

use messaging::{
    dto::{
        context::RequestContext,
        requests::channel::{
            abort_voice_connection::AbortVoiceConnectionRequest,
            join_voice_connection::JoinVoiceConnectionRequest,
            leave_voice_connection::LeaveVoiceConnectionRequest,
            track_media_connection::TrackMediaConnectionRequest,
        },
    },
    ports::event_consumer::{EventConsumer, StartPosition, SubscribeOptions},
    usecases::channel::{
        abort_voice_connection::{AbortVoiceConnection, AbortVoiceConnectionError},
        join_voice_connection::{JoinVoiceConnection, JoinVoiceConnectionError},
        leave_voice_connection::{LeaveVoiceConnection, LeaveVoiceConnectionError},
        track_media_connection::{TrackMediaConnection, TrackMediaConnectionError},
    },
};
use domain::{
    channel::events::participant::{ConnectionAbortReason, LeaveReason, MediaSource},
    channel::repository::ChannelRepository,
    shared::repository::error::RepositoryError,
};
use chrono::Utc;
use futures::StreamExt;
use tracing::Instrument;

fn to_domain_media_source(source: MediaSourceV1) -> MediaSource {
    match source {
        MediaSourceV1::Camera => MediaSource::Camera,
        MediaSourceV1::Screen => MediaSource::Screen,
    }
}
use messaging::messaging::nats::consumer::NatsEventConsumer;
use contracts::events::webhook::{MediaSourceV1, WebhookEventV1 as WebhookEventDto, WebhookSource};

pub fn spawn<T>(
    jetstream: async_nats::jetstream::Context,
    subject_prefix: String,
    stream: String,
    channel_repository: Arc<T>,
) where
    T: ChannelRepository + Send + Sync + 'static,
{
    actix_web::rt::spawn(async move {
        let prefix = subject_prefix.trim_end_matches('.').to_string();
        let consumer = NatsEventConsumer::new(jetstream, stream.clone());
        let join_uc = JoinVoiceConnection::new(channel_repository.clone());
        let abort_uc = AbortVoiceConnection::new(channel_repository.clone());
        let leave_uc = LeaveVoiceConnection::new(channel_repository.clone());
        let track_uc = TrackMediaConnection::new(channel_repository.clone());
        loop {
            let opts = SubscribeOptions {
                consumer_name: "messaging-voice-webhooks".to_string(),
                subject: format!("{prefix}.channel.>"),
                start_position: StartPosition::All,
                max_deliver: Some(5),
                dlq_subject: Some(format!("{prefix}.dlq.messaging.webhooks")),
                extra_subjects: vec![],
            };
            match consumer.subscribe(opts).await {
                Ok(mut messages) => {
                    tracing::info!(%stream, "Messaging voice webhook consumer subscribed");
                    while let Some(msg) = messages.next().await {
                        match msg {
                            Ok(m) => {
                                let kind = webhook_kind(&m.subject);
                                let Some(kind) = kind else {
                                    let _ = m.ack().await;
                                    continue;
                                };
                                let span = tracing::info_span!(
                                    "event",
                                    request_id = ?m.request_id,
                                    subject = %m.subject,
                                );
                                async {
                                    match apply(
                                        kind,
                                        &m.payload,
                                        m.request_id.clone(),
                                        &join_uc,
                                        &abort_uc,
                                        &leave_uc,
                                        &track_uc,
                                    )
                                    .await
                                    {
                                        Ok(()) => {
                                            if let Err(e) = m.ack().await {
                                                tracing::warn!(error = %e, "Webhook projection ack failed");
                                            }
                                        }
                                        Err(e) => {
                                            tracing::warn!(error = %e, subject = %m.subject, "Webhook projection failed; message will be redelivered");
                                        }
                                    }
                                }
                                .instrument(span)
                                .await;
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, "Webhook projection stream error");
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Webhook projection subscribe failed; retrying");
                }
            }
            actix_web::rt::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });
}

#[derive(Clone, Copy)]
enum WebhookKind {
    Joined,
    Aborted,
    Left,
    Track,
}

fn webhook_kind(subject: &str) -> Option<WebhookKind> {
    if subject.ends_with(".channel.webhook_joined") {
        Some(WebhookKind::Joined)
    } else if subject.ends_with(".channel.webhook_aborted") {
        Some(WebhookKind::Aborted)
    } else if subject.ends_with(".channel.webhook_left") {
        Some(WebhookKind::Left)
    } else if subject.ends_with(".channel.webhook_track") {
        Some(WebhookKind::Track)
    } else {
        None
    }
}

async fn apply<T>(
    kind: WebhookKind,
    payload: &[u8],
    request_id: Option<String>,
    join_uc: &JoinVoiceConnection<T>,
    abort_uc: &AbortVoiceConnection<T>,
    leave_uc: &LeaveVoiceConnection<T>,
    track_uc: &TrackMediaConnection<T>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    T: ChannelRepository + Send + Sync,
{
    let evt: WebhookEventDto = match serde_json::from_slice(payload) {
        Ok(evt) => evt,
        Err(e) => {
            tracing::warn!(error = %e, "Malformed webhook event; dropping");
            return Ok(());
        }
    };
    let Some(source) = WebhookSource::parse(&evt.inbox_source) else {
        tracing::warn!(source = %evt.inbox_source, "Unknown webhook source; dropping");
        return Ok(());
    };
    let ctx = RequestContext { request_id };

    match kind {
        WebhookKind::Joined => {
            let result = join_uc
                .execute(JoinVoiceConnectionRequest {
                    channel_id: evt.channel_id,
                    user_id: evt.user_id,
                    session_id: evt.session_id,
                    occurred_at: evt.occurred_at,
                    inbox_source: source,
                    inbox_external_id: evt.inbox_external_id,
                    ctx,
                })
                .await;
            match result {
                Ok(_) => Ok(()),
                Err(e) if is_join_stale(&e) => Ok(()),
                Err(e) => Err(e.into()),
            }
        }
        WebhookKind::Aborted => {
            let reason: ConnectionAbortReason = match evt
                .reason
                .and_then(|v| serde_json::from_value(v).ok())
            {
                Some(r) => r,
                None => {
                    tracing::warn!("Missing abort reason; dropping webhook event");
                    return Ok(());
                }
            };
            let result = abort_uc
                .execute(AbortVoiceConnectionRequest {
                    channel_id: evt.channel_id,
                    user_id: evt.user_id,
                    session_id: evt.session_id,
                    reason,
                    occurred_at: evt.occurred_at,
                    inbox_source: source,
                    inbox_external_id: evt.inbox_external_id,
                    ctx,
                })
                .await;
            match result {
                Ok(_) => Ok(()),
                Err(e) if is_abort_stale(&e) => Ok(()),
                Err(e) => Err(e.into()),
            }
        }
        WebhookKind::Left => {
            let reason: LeaveReason = match evt
                .reason
                .and_then(|v| serde_json::from_value(v).ok())
            {
                Some(r) => r,
                None => {
                    tracing::warn!("Missing leave reason; dropping webhook event");
                    return Ok(());
                }
            };
            let result = leave_uc
                .execute(LeaveVoiceConnectionRequest {
                    channel_id: evt.channel_id,
                    user_id: evt.user_id,
                    session_id: evt.session_id,
                    reason,
                    occurred_at: evt.occurred_at,
                    inbox_source: source,
                    inbox_external_id: evt.inbox_external_id,
                    ctx,
                })
                .await;
            match result {
                Ok(_) => Ok(()),
                Err(e) if is_leave_stale(&e) => Ok(()),
                Err(e) => Err(e.into()),
            }
        }
        WebhookKind::Track => {
            let (Some(media_source), Some(published)) = (evt.media_source, evt.published)
            else {
                tracing::warn!("Missing track fields; dropping webhook event");
                return Ok(());
            };
            let occurred_at = evt.occurred_at;
            let result = track_uc
                .execute(TrackMediaConnectionRequest {
                    channel_id: evt.channel_id,
                    user_id: evt.user_id,
                    session_id: evt.session_id,
                    source: to_domain_media_source(media_source),
                    published,
                    occurred_at,
                    inbox_source: source,
                    inbox_external_id: evt.inbox_external_id,
                    ctx,
                })
                .await;
            match result {
                Ok(_) => Ok(()),
                Err(TrackMediaConnectionError::Domain(
                    domain::channel::ChannelError::ParticipantNotJoined,
                )) => {
                    let expired = Utc::now().signed_duration_since(occurred_at)
                        >= chrono::Duration::seconds(TRACK_JOIN_GRACE_SECS);
                    if expired {
                        tracing::info!("Dropping track webhook that never saw its join");
                        Ok(())
                    } else {
                        Err("track event arrived before participant join; will retry".into())
                    }
                }
                Err(e) if is_track_stale(&e) => Ok(()),
                Err(e) => Err(e.into()),
            }
        }
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
