use serde::Serialize;

use crate::room::{
    events::{
        participant_events::{ParticipantJoinedEvent, ParticipantLeftEvent},
        room_events::{RoomFinishedEvent, RoomStartedEvent},
        track_events::{TrackPublishedEvent, TrackUnpublishedEvent},
    },
    value_objects::id::RoomId,
};

pub mod participant_events;
pub mod room_events;
pub mod track_events;

#[derive(Debug, Serialize)]
pub enum RoomEvent {
    RoomStarted(RoomStartedEvent),
    RoomFinished(RoomFinishedEvent),

    ParticipantJoined(ParticipantJoinedEvent),
    ParticipantLeft(ParticipantLeftEvent),

    TrackPublished(TrackPublishedEvent),
    TrackUnpublished(TrackUnpublishedEvent),
}

impl RoomEvent {
    pub fn room_id(&self) -> &RoomId {
        match self {
            RoomEvent::RoomStarted(event) => event.room_id(),
            RoomEvent::RoomFinished(event) => event.room_id(),
            RoomEvent::ParticipantJoined(event) => event.room_id(),
            RoomEvent::ParticipantLeft(event) => event.room_id(),
            RoomEvent::TrackPublished(event) => event.room_id(),
            RoomEvent::TrackUnpublished(event) => event.room_id(),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            RoomEvent::RoomStarted(_) => "room.started",
            RoomEvent::RoomFinished(_) => "room.finished",
            RoomEvent::ParticipantJoined(_) => "participant.joined",
            RoomEvent::ParticipantLeft(_) => "participant.left",
            RoomEvent::TrackPublished(_) => "track.published",
            RoomEvent::TrackUnpublished(_) => "track.unpublished",
        }
    }
}
