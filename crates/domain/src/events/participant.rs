use chrono::{DateTime, Utc};

use crate::{entites::paticipant::Participant, value_objects::room::name::RoomName};

#[derive(Debug)]
pub struct ParticipantJoinedEvent {
    room_name: RoomName,
    participant: Participant,
    occurred_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct ParticipantLeftEvent {
    room_name: RoomName,
    participant: Participant,
    occurred_at: DateTime<Utc>,
}
