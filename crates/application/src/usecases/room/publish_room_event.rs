use std::sync::Arc;

use domain::room::events::RoomEvent;

use crate::ports::event_producer::{EventProducer, EventProducerError};

#[derive(Debug, thiserror::Error)]
pub enum PublishRoomEventError {
    #[error("Event producer error: {0}")]
    EventProducer(#[from] EventProducerError),
}

pub struct PublishRoomEvent<T>
where
    T: EventProducer<RoomEvent>,
{
    event_producer: Arc<T>,
}

impl<T> PublishRoomEvent<T>
where
    T: EventProducer<RoomEvent>,
{
    pub fn new(event_producer: Arc<T>) -> Self {
        Self { event_producer }
    }

    pub async fn execute(&self, event: RoomEvent) -> Result<(), PublishRoomEventError> {
        self.event_producer.publish(&event).await?;
        Ok(())
    }
}
