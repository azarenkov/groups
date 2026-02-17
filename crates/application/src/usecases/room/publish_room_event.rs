use std::sync::Arc;

use domain::{
    room::{events::RoomEvent, repository::RoomRepository},
    shared::repository::error::RepositoryError,
};

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

pub struct PublishRoomEvent<T, U>
where
    T: EventProducer<RoomEvent>,
    U: RoomRepository,
{
    event_producer: Arc<T>,
    room_repository: Arc<U>,
}

impl<T, U> PublishRoomEvent<T, U>
where
    T: EventProducer<RoomEvent>,
    U: RoomRepository,
{
    pub fn new(event_producer: Arc<T>, room_repository: Arc<U>) -> Self {
        Self {
            event_producer,
            room_repository,
        }
    }

    pub async fn execute(&self, dto: RoomEventDto) -> Result<(), PublishRoomEventError> {
        let room_id = EventMapper::extract_room_id(&dto)?;

        let room = self.room_repository.get_room_by_id(&room_id).await?;

        let event = EventMapper::to_domain_event(dto, &room)?;

        self.event_producer.publish(&event).await?;

        Ok(())
    }
}
