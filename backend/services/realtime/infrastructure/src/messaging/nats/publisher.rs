use async_nats::Client;
use async_trait::async_trait;
use contracts::realtime::PresencePreferenceV1;
use realtime_core::ports::publisher::{PublishError, RealtimePublisher};
use realtime_domain::envelope::Envelope;
use uuid::Uuid;

#[derive(Clone)]
pub struct NatsRealtimePublisher {
    core: Client,
    user_subject_prefix: String,
    presence_preference_subject: String,
}

impl NatsRealtimePublisher {
    pub fn new(core: Client, user_subject_prefix: String, presence_preference_subject: String) -> Self {
        Self {
            core,
            user_subject_prefix,
            presence_preference_subject,
        }
    }
}

#[async_trait]
impl RealtimePublisher for NatsRealtimePublisher {
    async fn publish_to_users(&self, recipients: Vec<Uuid>, envelope: &Envelope<'_>) -> Result<(), PublishError> {
        let payload = serde_json::to_vec(envelope).map_err(PublishError::Serialize)?;

        // Best-effort fanout. If a per-recipient publish fails we log it but DO NOT
        // surface the error, otherwise the JetStream consumer would refuse to ack
        // and the whole event would be redelivered, causing duplicates for every
        // recipient whose publish succeeded the first time.
        let mut failures = 0usize;
        let mut total = 0usize;
        for user_id in recipients {
            total += 1;
            let subject = format!("{}.{}", self.user_subject_prefix, user_id);
            if let Err(e) = self
                .core
                .publish(subject.clone(), payload.clone().into())
                .await
            {
                failures += 1;
                tracing::warn!(error = %e, %subject, "Failed to publish to user subject");
            }
        }
        if failures > 0 {
            tracing::warn!(
                failed = failures,
                total,
                "Some realtime fanout publishes failed; acking source event anyway to avoid duplicate redelivery",
            );
        }
        Ok(())
    }

    async fn announce_presence_preference(&self, user_id: Uuid, appear_offline: bool) -> Result<(), PublishError> {
        let payload = serde_json::to_vec(&PresencePreferenceV1 { user_id, appear_offline }).map_err(PublishError::Serialize)?;
        self.core
            .publish(self.presence_preference_subject.clone(), payload.into())
            .await
            .map_err(|e| PublishError::Transport(e.to_string()))?;
        self.core.flush().await.map_err(|e| PublishError::Transport(e.to_string()))
    }
}
