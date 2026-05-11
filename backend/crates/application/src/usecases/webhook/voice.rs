use std::sync::Arc;

use domain::{channel::repository::ChannelRepository, shared::repository::error::RepositoryError};

use crate::{
    dto::requests::channel::{
        abort_voice_connection::AbortVoiceConnectionRequest,
        leave_voice_connection::LeaveVoiceConnectionRequest,
    },
    ports::{
        voice_webhook_verifier::{
            VoiceWebhookEvent, VoiceWebhookVerifier, VoiceWebhookVerifierError,
        },
        webhook_inbox::{WebhookInbox, WebhookSource},
    },
    usecases::channel::{
        abort_voice_connection::{AbortVoiceConnection, AbortVoiceConnectionError},
        leave_voice_connection::{LeaveVoiceConnection, LeaveVoiceConnectionError},
    },
};

#[derive(Debug, thiserror::Error)]
pub enum HandleVoiceWebhookError {
    #[error("Webhook verifier error: {0}")]
    Verifier(#[from] VoiceWebhookVerifierError),
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Abort voice connection error: {0}")]
    AbortVoiceConnection(#[from] AbortVoiceConnectionError),
    #[error("Leave voice connection error: {0}")]
    LeaveVoiceConnection(#[from] LeaveVoiceConnectionError),
}

pub struct HandleVoiceWebhook<V, T, I>
where
    V: VoiceWebhookVerifier,
    T: ChannelRepository,
    I: WebhookInbox,
{
    verifier: Arc<V>,
    inbox: Arc<I>,
    abort_voice_connection: AbortVoiceConnection<T>,
    leave_voice_connection: LeaveVoiceConnection<T>,
}

impl<V, T, I> HandleVoiceWebhook<V, T, I>
where
    V: VoiceWebhookVerifier,
    T: ChannelRepository,
    I: WebhookInbox,
{
    pub fn new(
        verifier: Arc<V>,
        inbox: Arc<I>,
        abort_voice_connection: AbortVoiceConnection<T>,
        leave_voice_connection: LeaveVoiceConnection<T>,
    ) -> Self {
        Self {
            verifier,
            inbox,
            abort_voice_connection,
            leave_voice_connection,
        }
    }

    pub async fn execute(
        &self,
        body: &str,
        auth_token: &str,
    ) -> Result<(), HandleVoiceWebhookError> {
        let envelope = self.verifier.verify(body, auth_token)?;

        if !self
            .inbox
            .try_record_processed(WebhookSource::LiveKit, &envelope.external_id)
            .await?
        {
            return Ok(());
        }

        let occurred_at = envelope.occurred_at;

        match envelope.event {
            VoiceWebhookEvent::ParticipantConnectionAborted {
                channel_id,
                user_id,
                reason,
            } => {
                self.abort_voice_connection
                    .execute(AbortVoiceConnectionRequest {
                        channel_id,
                        user_id,
                        reason,
                        occurred_at,
                    })
                    .await?;
            }
            VoiceWebhookEvent::ParticipantLeft {
                channel_id,
                user_id,
                reason,
            } => {
                self.leave_voice_connection
                    .execute(LeaveVoiceConnectionRequest {
                        channel_id,
                        user_id,
                        reason,
                        occurred_at,
                    })
                    .await?;
            }
            VoiceWebhookEvent::Other => {}
        }

        Ok(())
    }
}
