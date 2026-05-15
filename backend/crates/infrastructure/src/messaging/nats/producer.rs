use application::ports::outbox_publisher::{OutboxPublisher, OutboxPublisherError};
use async_nats::{HeaderMap, jetstream::Context};
use async_trait::async_trait;
use domain::shared::outbox::outbox_record::OutboxRecord;

const MSG_ID_HEADER: &str = "Nats-Msg-Id";
const REQUEST_ID_HEADER: &str = "X-Request-Id";

pub struct NatsOutboxPublisher {
    jetstream: Context,
    subject_prefix: String,
}

impl NatsOutboxPublisher {
    pub fn new(jetstream: Context, subject_prefix: String) -> Self {
        Self {
            jetstream,
            subject_prefix,
        }
    }

    fn subject_for(&self, record: &OutboxRecord) -> String {
        format!(
            "{}.{}.{}",
            self.subject_prefix,
            record.aggregate_type.as_str(),
            record.event_type,
        )
    }
}

#[async_trait]
impl OutboxPublisher for NatsOutboxPublisher {
    async fn publish(&self, record: &OutboxRecord) -> Result<(), OutboxPublisherError> {
        let subject = self.subject_for(record);
        let payload = serde_json::to_vec(&record.payload)
            .map_err(|e| OutboxPublisherError::Serialization(e.to_string()))?;

        let mut headers = HeaderMap::new();
        headers.insert(MSG_ID_HEADER, record.id.to_string().as_str());
        if let Some(request_id) = record.request_id.as_deref() {
            headers.insert(REQUEST_ID_HEADER, request_id);
        }

        let ack_future = self
            .jetstream
            .publish_with_headers(subject, headers, payload.into())
            .await
            .map_err(|e| OutboxPublisherError::Publish(e.to_string()))?;

        ack_future
            .await
            .map_err(|e| OutboxPublisherError::Ack(e.to_string()))?;

        Ok(())
    }
}
