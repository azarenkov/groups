use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{room::value_objects::id::RoomId, server::value_objects::id::ServerId};

#[derive(Debug, Serialize)]
pub struct RoomStartedEvent {
    id: String,
    room_id: RoomId,
    server_id: ServerId,
    created_at: DateTime<Utc>,
}

impl RoomStartedEvent {
    pub fn new(
        id: String,
        room_id: RoomId,
        server_id: ServerId,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            room_id,
            server_id,
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
    server_id: ServerId,
    created_at: DateTime<Utc>,
}

impl RoomFinishedEvent {
    pub fn new(
        id: String,
        room_id: RoomId,
        server_id: ServerId,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            room_id,
            server_id,
            created_at,
        }
    }

    pub fn room_id(&self) -> &RoomId {
        &self.room_id
    }
}
