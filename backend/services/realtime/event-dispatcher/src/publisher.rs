use async_nats::Client;
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone)]
pub struct Publisher {
    core: Client,
    user_subject_prefix: String,
}

impl Publisher {
    pub fn new(core: Client, user_subject_prefix: String) -> Self {
        Self {
            core,
            user_subject_prefix,
        }
    }

    pub async fn publish_to_users<I, E>(
        &self,
        recipients: I,
        envelope: &E,
    ) -> Result<(), PublishError>
    where
        I: IntoIterator<Item = Uuid>,
        E: Serialize,
    {
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
}

#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    #[error("serialize envelope: {0}")]
    Serialize(serde_json::Error),
}
