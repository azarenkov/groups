use crate::dto::requests::room::events::{
    participant_events::ParticipantEventRequest, room_events::RoomEventRequest,
    track_events::TrackEventRequest,
};

pub mod participant_events;
pub mod room_events;
pub mod track_events;

pub enum RoomEventDto {
    RoomStarted(RoomEventRequest),
    RoomFinished(RoomEventRequest),

    ParticipantJoined(ParticipantEventRequest),
    ParticipantLeft(ParticipantEventRequest),
    ParticipantConnectionAborted(ParticipantEventRequest),

    TrackPublished(TrackEventRequest),
    TrackUnpublished(TrackEventRequest),
}
