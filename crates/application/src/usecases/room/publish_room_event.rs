use std::sync::Arc;

use domain::{room::events::RoomEvent, shared::repository::error::RepositoryError};

use crate::{
    dto::requests::room::events::RoomEventDto,
    mappers::room_event::{EventMapper, EventMapperError},
    ports::event_producer::{EventProducer, EventProducerError},
};

#[derive(Debug, thiserror::Error)]
pub enum PublishRoomEventError {
    #[error("Event producer error: {0}")]
    EventProducer(#[from] EventProducerError),
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Event mapper error: {0}")]
    EventMapper(#[from] EventMapperError),
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

    pub async fn execute(&self, dto: RoomEventDto) -> Result<(), PublishRoomEventError> {
        let event = EventMapper::to_domain_event(dto)?;
        let room_id = event.room_id().uuid().to_string();

        self.event_producer.publish(&event).await?;

        if let RoomEvent::RoomFinished(_) = &event {
            self.event_producer.delete_events_by_id(room_id).await?;
        }

        Ok(())
    }
}
