use std::sync::Arc;

use domain::user::id::UserId;
use ports::repository::RepositoryError;
use voice_domain::{call::repository::CallRepository, webhook::WebhookEvent, webhook::WebhookKind};

use crate::{
    dto::context::RequestContext,
    ports::{
        voice_webhook_verifier::{VoiceWebhookEvent, VoiceWebhookVerifier, VoiceWebhookVerifierError},
        webhook_outbox::WebhookOutbox,
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
    O: WebhookOutbox,
{
    verifier: Arc<V>,
    call_repository: Arc<C>,
    outbox: Arc<O>,
}

impl<V, C, O> HandleVoiceWebhook<V, C, O>
where
    V: VoiceWebhookVerifier,
    C: CallRepository,
    O: WebhookOutbox,
{
    pub fn new(verifier: Arc<V>, call_repository: Arc<C>, outbox: Arc<O>) -> Self {
        Self {
            verifier,
            call_repository,
            outbox,
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

        let (channel_id, user_id, session_id, kind) = match envelope.event {
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
                (channel_id, user_id, session_id, WebhookKind::Joined)
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
                (channel_id, user_id, session_id, WebhookKind::Aborted(reason))
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
                (channel_id, user_id, session_id, WebhookKind::Left(reason))
            }
            VoiceWebhookEvent::TrackPublished {
                channel_id,
                user_id,
                session_id,
                source: media_source,
            } => {
                if self.call_repository.is_call_room(channel_id).await? {
                    return Ok(());
                }
                (
                    channel_id,
                    user_id,
                    session_id,
                    WebhookKind::Track {
                        source: media_source,
                        published: true,
                    },
                )
            }
            VoiceWebhookEvent::TrackUnpublished {
                channel_id,
                user_id,
                session_id,
                source: media_source,
            } => {
                if self.call_repository.is_call_room(channel_id).await? {
                    return Ok(());
                }
                (
                    channel_id,
                    user_id,
                    session_id,
                    WebhookKind::Track {
                        source: media_source,
                        published: false,
                    },
                )
            }
            VoiceWebhookEvent::Other => return Ok(()),
        };

        let event = WebhookEvent {
            source,
            external_id,
            channel_id,
            user_id,
            session_id,
            occurred_at,
            kind,
        };
        self.outbox.save(&event, ctx.request_id.as_deref()).await?;
        Ok(())
    }
}
