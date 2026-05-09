use serde::Serialize;
use uuid::Uuid;

use crate::{
    room::{
        events::{
            participant_events::{
                ParticipantConnectionAbortedEvent, ParticipantJoinedEvent, ParticipantLeftEvent,
            },
            room_events::{RoomFinishedEvent, RoomStartedEvent},
            track_events::{TrackPublishedEvent, TrackUnpublishedEvent},
        },
        id::RoomId,
    },
    server::id::ServerId,
    shared::event::DomainEvent,
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
    ParticipantConnectionAborted(ParticipantConnectionAbortedEvent),

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
            RoomEvent::ParticipantConnectionAborted(event) => event.room_id(),
        }
    }

    pub fn server_id(&self) -> Option<&ServerId> {
        match self {
            RoomEvent::RoomStarted(event) => event.server_id().as_ref(),
            RoomEvent::RoomFinished(event) => event.server_id().as_ref(),
            RoomEvent::ParticipantJoined(event) => event.server_id().as_ref(),
            RoomEvent::ParticipantLeft(event) => event.server_id().as_ref(),
            RoomEvent::TrackPublished(event) => event.server_id().as_ref(),
            RoomEvent::TrackUnpublished(event) => event.server_id().as_ref(),
            RoomEvent::ParticipantConnectionAborted(event) => event.server_id().as_ref(),
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
            RoomEvent::ParticipantConnectionAborted(_) => "participant.aborted",
        }
    }
}

impl DomainEvent for RoomEvent {
    fn event_category(&self) -> &'static str {
        "room"
    }

    fn event_type(&self) -> &'static str {
        match self {
            RoomEvent::RoomStarted(_) => "started",
            RoomEvent::RoomFinished(_) => "finished",
            RoomEvent::ParticipantJoined(_) => "participant.joined",
            RoomEvent::ParticipantLeft(_) => "participant.left",
            RoomEvent::TrackPublished(_) => "track.published",
            RoomEvent::TrackUnpublished(_) => "track.unpublished",
            RoomEvent::ParticipantConnectionAborted(_) => "participant.aborted",
        }
    }

    fn aggregate_id(&self) -> Uuid {
        self.room_id().uuid()
    }

    fn parent_id(&self) -> Option<Uuid> {
        self.server_id().map(|id| id.uuid())
    }
}
