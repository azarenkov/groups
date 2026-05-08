use application::ports::event_producer::{EventProducer, EventProducerError};
use async_nats::jetstream::Context;
use async_trait::async_trait;
use domain::shared::event::DomainEvent;

use crate::{config::nats::NatsConfig, messaging::nats::connection::STREAM_NAME};

pub struct NatsProducer {
    jetstream: Context,
    subject_prefix: String,
}

impl NatsProducer {
    pub fn new(jetstream: Context, nats_config: NatsConfig) -> Self {
        Self {
            jetstream,
            subject_prefix: nats_config.subject_prefix,
        }
    }

    fn build_subject<T: DomainEvent>(&self, event: &T) -> String {
        if let Some(parent_id) = event.parent_id() {
            format!(
                "{}.server.{}.{}.{}.{}",
                self.subject_prefix,
                parent_id,
                event.event_category(),
                event.aggregate_id(),
                event.event_type()
            )
        } else {
            format!(
                "{}.{}.{}.{}",
                self.subject_prefix,
                event.event_category(),
                event.aggregate_id(),
                event.event_type()
            )
        }
    }
}

#[async_trait]
impl<T> EventProducer<T> for NatsProducer
where
    T: DomainEvent + Send + Sync,
{
    async fn publish(&self, event: &T) -> Result<(), EventProducerError> {
        let subject = self.build_subject(event);

        let payload = serde_json::to_vec(event)
            .map_err(|e| EventProducerError::Serialization(e.to_string()))?;

        let ack = self
            .jetstream
            .publish(subject, payload.into())
            .await
            .map_err(|e| EventProducerError::Publish(e.to_string()))?;

        ack.await
            .map_err(|e| EventProducerError::Ack(e.to_string()))?;

        Ok(())
    }

    async fn delete_events_by_id(&self, id: String) -> Result<(), EventProducerError> {
        let subject_filter = format!("{}.*.{}.*", self.subject_prefix, id);

        let stream = self
            .jetstream
            .get_stream(STREAM_NAME.to_string())
            .await
            .map_err(|e| EventProducerError::Publish(format!("Failed to get stream: {}", e)))?;

        stream
            .purge()
            .filter(&subject_filter)
            .await
            .map_err(|e| EventProducerError::Publish(format!("Failed to purge events: {}", e)))?;

        Ok(())
    }
}
