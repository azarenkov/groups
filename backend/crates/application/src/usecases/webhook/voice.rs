use std::sync::Arc;

use domain::{channel::repository::ChannelRepository, shared::repository::error::RepositoryError};

use crate::{
    dto::requests::channel::{
        abort_voice_connection::AbortVoiceConnectionRequest,
        join_voice_connection::JoinVoiceConnectionRequest,
        leave_voice_connection::LeaveVoiceConnectionRequest,
    },
    ports::voice_webhook_verifier::{
        VoiceWebhookEvent, VoiceWebhookVerifier, VoiceWebhookVerifierError,
    },
    usecases::channel::{
        abort_voice_connection::{AbortVoiceConnection, AbortVoiceConnectionError},
        join_voice_connection::{JoinVoiceConnection, JoinVoiceConnectionError},
        leave_voice_connection::{LeaveVoiceConnection, LeaveVoiceConnectionError},
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
}

pub struct HandleVoiceWebhook<V, T>
where
    V: VoiceWebhookVerifier,
    T: ChannelRepository,
{
    verifier: Arc<V>,
    join_voice_connection: JoinVoiceConnection<T>,
    abort_voice_connection: AbortVoiceConnection<T>,
    leave_voice_connection: LeaveVoiceConnection<T>,
}

impl<V, T> HandleVoiceWebhook<V, T>
where
    V: VoiceWebhookVerifier,
    T: ChannelRepository,
{
    pub fn new(
        verifier: Arc<V>,
        join_voice_connection: JoinVoiceConnection<T>,
        abort_voice_connection: AbortVoiceConnection<T>,
        leave_voice_connection: LeaveVoiceConnection<T>,
    ) -> Self {
        Self {
            verifier,
            join_voice_connection,
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

        let source = envelope.source;
        let external_id = envelope.external_id;
        let occurred_at = envelope.occurred_at;

        match envelope.event {
            VoiceWebhookEvent::ParticipantJoined {
                channel_id,
                user_id,
                session_id,
            } => {
                let result = self
                    .join_voice_connection
                    .execute(JoinVoiceConnectionRequest {
                        channel_id,
                        user_id,
                        session_id,
                        occurred_at,
                        inbox_source: source,
                        inbox_external_id: external_id.clone(),
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
            VoiceWebhookEvent::Other => {}
        }

        Ok(())
    }
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
