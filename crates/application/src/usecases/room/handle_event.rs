use crate::{
    dto::room::event::{RoomEvent, RoomEventNotification, RoomEventType},
    ports::room_event_bus::{RoomEventBus, RoomEventBusError},
};

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

    pub async fn execute(&self, event: RoomEvent) -> Result<(), RoomEventError> {
        let notification = match event {
            RoomEvent::ParticipantJoined { room, participant } => RoomEventNotification {
                event_type: RoomEventType::ParticipantJoined,
                room_name: room.name,
                participant,
            },
            RoomEvent::ParticipantLeft { room, participant } => RoomEventNotification {
                event_type: RoomEventType::ParticipantLeft,
                room_name: room.name,
                participant,
            },
        };

        self.event_bus.publish_event(notification).await?;

        Ok(())
    }
}
