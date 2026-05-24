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

        let mut last_err: Option<async_nats::PublishError> = None;
        for user_id in recipients {
            let subject = format!("{}.{}", self.user_subject_prefix, user_id);
            if let Err(e) = self
                .core
                .publish(subject.clone(), payload.clone().into())
                .await
            {
                tracing::warn!(error = %e, %subject, "Failed to publish to user subject");
                last_err = Some(e);
            }
        }
        match last_err {
            None => Ok(()),
            Some(e) => Err(PublishError::Nats(e)),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    #[error("serialize envelope: {0}")]
    Serialize(serde_json::Error),
    #[error("nats publish: {0}")]
    Nats(async_nats::PublishError),
}
