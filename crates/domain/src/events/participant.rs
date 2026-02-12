use chrono::{DateTime, Utc};

use crate::{entites::paticipant::Participant, value_objects::room::name::RoomName};

#[derive(Debug)]
pub struct ParticipantJoinedEvent {
    room_name: RoomName,
    participant: Participant,
    occurred_at: DateTime<Utc>,
}

impl ParticipantJoinedEvent {
    pub fn new(room_name: RoomName, participant: Participant, occurred_at: DateTime<Utc>) -> Self {
        Self {
            room_name,
            participant,
            occurred_at,
        }
    }
}

#[derive(Debug)]
pub struct ParticipantLeftEvent {
    room_name: RoomName,
    participant: Participant,
    occurred_at: DateTime<Utc>,
}

impl ParticipantLeftEvent {
    pub fn new(room_name: RoomName, participant: Participant, occurred_at: DateTime<Utc>) -> Self {
        Self {
            room_name,
            participant,
            occurred_at,
        }
    }
}
