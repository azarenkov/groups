use application::ports::event_producer::{EventProducer, EventProducerError};
use async_nats::jetstream::Context;
use async_trait::async_trait;
use domain::room::events::RoomEvent;

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

    fn build_subject(&self, event: &RoomEvent) -> String {
        let event_type = event.as_str();
        let room_id = event.room_id().uuid();

        format!("{}.{}.{}", self.subject_prefix, room_id, event_type)
    }
}

#[async_trait]
impl EventProducer<RoomEvent> for NatsProducer {
    async fn publish(&self, event: &RoomEvent) -> Result<(), EventProducerError> {
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
        let subject_filter = format!("{}.{}.*", self.subject_prefix, id);

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
