use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{
    room::value_objects::id::RoomId,
    server::value_objects::id::ServerId,
    shared::value_objects::track_type::TrackType,
    user::value_objects::{id::UserId, name::UserName},
};

#[derive(Debug, Serialize)]
pub struct TrackPublishedEvent {
    id: String,
    user_id: UserId,
    user_name: UserName,
    room_id: RoomId,
    server_id: ServerId,
    track_type: TrackType,
    created_at: DateTime<Utc>,
}

impl TrackPublishedEvent {
    pub fn new(
        id: String,
        user_id: UserId,
        user_name: UserName,
        room_id: RoomId,
        server_id: ServerId,
        track_type: TrackType,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            user_id,
            user_name,
            room_id,
            server_id,
            track_type,
            created_at,
        }
    }

    pub fn room_id(&self) -> &RoomId {
        &self.room_id
    }
}

#[derive(Debug, Serialize)]
pub struct TrackUnpublishedEvent {
    id: String,
    user_id: UserId,
    user_name: UserName,
    room_id: RoomId,
    server_id: ServerId,
    track_type: TrackType,
    created_at: DateTime<Utc>,
}

impl TrackUnpublishedEvent {
    pub fn new(
        id: String,
        user_id: UserId,
        user_name: UserName,
        room_id: RoomId,
        server_id: ServerId,
        track_type: TrackType,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            user_id,
            user_name,
            room_id,
            server_id,
            track_type,
            created_at,
        }
    }

    pub fn room_id(&self) -> &RoomId {
        &self.room_id
    }
}
