use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{
    room::value_objects::{id::RoomId, room_type::RoomType},
    server::value_objects::id::ServerId,
    user::value_objects::id::UserId,
};

#[derive(Debug, Serialize)]
pub struct RoomStartedEvent {
    id: String,
    room_id: RoomId,
    owner_id: UserId,
    server_id: ServerId,
    room_type: RoomType,
    created_at: DateTime<Utc>,
}

impl RoomStartedEvent {
    pub fn new(
        id: String,
        room_id: RoomId,
        owner_id: UserId,
        server_id: ServerId,
        room_type: RoomType,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            room_id,
            owner_id,
            server_id,
            room_type,
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
    owner_id: UserId,
    server_id: ServerId,
    room_type: RoomType,
    created_at: DateTime<Utc>,
}

impl RoomFinishedEvent {
    pub fn new(
        id: String,
        room_id: RoomId,
        owner_id: UserId,
        server_id: ServerId,
        room_type: RoomType,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            room_id,
            owner_id,
            server_id,
            room_type,
            created_at,
        }
    }

    pub fn room_id(&self) -> &RoomId {
        &self.room_id
    }
}
