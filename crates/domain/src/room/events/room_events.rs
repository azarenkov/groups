use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::room::value_objects::id::RoomId;

#[derive(Debug, Serialize)]
pub struct RoomStartedEvent {
    id: String,
    room_id: RoomId,
    created_at: DateTime<Utc>,
}

impl RoomStartedEvent {
    pub fn new(id: String, room_id: RoomId, created_at: DateTime<Utc>) -> Self {
        Self {
            id,
            room_id,
            created_at,
        }
    }

    pub fn room_id(&self) -> &RoomId {
        &self.room_id
    }
}

#[derive(Debug, Serialize)]
pub struct RoomFinishedEvent {
    id: String,
    room_id: RoomId,
    created_at: DateTime<Utc>,
}

impl RoomFinishedEvent {
    pub fn new(id: String, room_id: RoomId, created_at: DateTime<Utc>) -> Self {
        Self {
            id,
            room_id,
            created_at,
        }
    }

    pub fn room_id(&self) -> &RoomId {
        &self.room_id
    }
}
