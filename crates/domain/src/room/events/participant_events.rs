use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{room::value_objects::id::RoomId, user::value_objects::id::UserId};

#[derive(Debug, Serialize)]
pub struct ParticipantJoinedEvent {
    id: String,
    user_id: UserId,
    room_id: RoomId,
    joined_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl ParticipantJoinedEvent {
    pub fn new(
        id: String,
        user_id: UserId,
        room_id: RoomId,
        joined_at: DateTime<Utc>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            user_id,
            room_id,
            joined_at,
            created_at,
        }
    }

    pub fn room_id(&self) -> &RoomId {
        &self.room_id
    }
}

#[derive(Debug, Serialize)]
pub struct ParticipantLeftEvent {
    id: String,
    user_id: UserId,
    room_id: RoomId,
    joined_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl ParticipantLeftEvent {
    pub fn new(
        id: String,
        user_id: UserId,
        room_id: RoomId,
        joined_at: DateTime<Utc>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            user_id,
            room_id,
            joined_at,
            created_at,
        }
    }

    pub fn room_id(&self) -> &RoomId {
        &self.room_id
    }
}
