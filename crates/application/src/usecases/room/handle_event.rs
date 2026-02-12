use crate::ports::room_event_bus::{RoomEventBus, RoomEventBusError};

#[derive(Debug, thiserror::Error)]
pub enum RoomEventError {
    #[error("Sending event error: {0}")]
    PublishEvent(#[from] RoomEventBusError),
}

pub struct HandleParticipantEvent<T>
where
    T: RoomEventBus,
{
    event_bus: T,
}

impl<T> HandleParticipantEvent<T>
where
    T: RoomEventBus,
{
    pub fn new(event_bus: T) -> Self {
        Self { event_bus }
    }

    // pub async fn execute(&self, event: RoomEventRequest) -> Result<(), RoomEventError> {
    //     self.event_bus.publish_event(notification).await?;

    //     Ok(())
    // }
}
