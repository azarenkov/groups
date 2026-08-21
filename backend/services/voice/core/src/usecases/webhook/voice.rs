use std::sync::Arc;

use chrono::{DateTime, Utc};
use domain::{
    call::repository::CallRepository,
    channel::events::participant::MediaSource,
    shared::{
        outbox::outbox_record::{AggregateType, OutboxRecord},
        repository::error::RepositoryError,
    },
    user::id::UserId,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    dto::context::RequestContext,
    ports::{
        outbox_writer::OutboxWriter,
        voice_webhook_verifier::{
            VoiceWebhookEvent, VoiceWebhookVerifier, VoiceWebhookVerifierError, WebhookSource,
        },
    },
};

#[derive(Debug, thiserror::Error)]
pub enum HandleVoiceWebhookError {
    #[error("Webhook verifier error: {0}")]
    Verifier(#[from] VoiceWebhookVerifierError),
    #[error("Call repository error: {0}")]
    Call(#[from] RepositoryError),
}

pub struct HandleVoiceWebhook<V, C, O>
where
    V: VoiceWebhookVerifier,
    C: CallRepository,
    O: OutboxWriter,
{
    verifier: Arc<V>,
    call_repository: Arc<C>,
    outbox_writer: Arc<O>,
}

impl<V, C, O> HandleVoiceWebhook<V, C, O>
where
    V: VoiceWebhookVerifier,
    C: CallRepository,
    O: OutboxWriter,
{
    pub fn new(verifier: Arc<V>, call_repository: Arc<C>, outbox_writer: Arc<O>) -> Self {
        Self {
            verifier,
            call_repository,
            outbox_writer,
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
                self.forward(
                    "webhook_joined",
                    channel_id,
                    json!({
                        "channel_id": channel_id,
                        "user_id": user_id,
                        "session_id": session_id,
                    }),
                    occurred_at,
                    source,
                    &external_id,
                    &ctx,
                )
                .await?;
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
                self.forward(
                    "webhook_aborted",
                    channel_id,
                    json!({
                        "channel_id": channel_id,
                        "user_id": user_id,
                        "session_id": session_id,
                        "reason": reason,
                    }),
                    occurred_at,
                    source,
                    &external_id,
                    &ctx,
                )
                .await?;
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
                self.forward(
                    "webhook_left",
                    channel_id,
                    json!({
                        "channel_id": channel_id,
                        "user_id": user_id,
                        "session_id": session_id,
                        "reason": reason,
                    }),
                    occurred_at,
                    source,
                    &external_id,
                    &ctx,
                )
                .await?;
            }
            VoiceWebhookEvent::TrackPublished {
                channel_id,
                user_id,
                session_id,
                source: media_source,
            } => {
                self.forward_track(
                    channel_id,
                    user_id,
                    session_id,
                    media_source,
                    true,
                    occurred_at,
                    source,
                    &external_id,
                    &ctx,
                )
                .await?;
            }
            VoiceWebhookEvent::TrackUnpublished {
                channel_id,
                user_id,
                session_id,
                source: media_source,
            } => {
                self.forward_track(
                    channel_id,
                    user_id,
                    session_id,
                    media_source,
                    false,
                    occurred_at,
                    source,
                    &external_id,
                    &ctx,
                )
                .await?;
            }
            VoiceWebhookEvent::Other => {}
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn forward_track(
        &self,
        channel_id: Uuid,
        user_id: Uuid,
        session_id: String,
        media_source: MediaSource,
        published: bool,
        occurred_at: DateTime<Utc>,
        source: WebhookSource,
        external_id: &str,
        ctx: &RequestContext,
    ) -> Result<(), HandleVoiceWebhookError> {
        if self.call_repository.is_call_room(channel_id).await? {
            return Ok(());
        }
        self.forward(
            "webhook_track",
            channel_id,
            json!({
                "channel_id": channel_id,
                "user_id": user_id,
                "session_id": session_id,
                "media_source": media_source,
                "published": published,
            }),
            occurred_at,
            source,
            external_id,
            ctx,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn forward(
        &self,
        event_type: &str,
        channel_id: Uuid,
        mut payload: serde_json::Value,
        occurred_at: DateTime<Utc>,
        source: WebhookSource,
        external_id: &str,
        ctx: &RequestContext,
    ) -> Result<(), HandleVoiceWebhookError> {
        if let serde_json::Value::Object(obj) = &mut payload {
            obj.insert("occurred_at".to_string(), json!(occurred_at));
            obj.insert("inbox_source".to_string(), json!(source.as_str()));
            obj.insert("inbox_external_id".to_string(), json!(external_id));
        }
        let record = OutboxRecord {
            id: Uuid::new_v4(),
            aggregate_id: channel_id,
            aggregate_type: AggregateType::Channel,
            parent_id: None,
            parent_type: None,
            event_type: event_type.to_string(),
            payload,
            occurred_at,
            request_id: ctx.request_id.clone(),
        };
        self.outbox_writer.save(&[record]).await?;
        Ok(())
    }
}
