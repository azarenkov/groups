use crate::events::participant::{ParticipantJoinedEvent, ParticipantLeftEvent};

#[derive(Debug)]
pub enum RoomEvent {
    ParticipantJoined(ParticipantJoinedEvent),
    ParticipantLeft(ParticipantLeftEvent),
}
