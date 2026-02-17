use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{
    room::value_objects::id::RoomId, shared::value_objects::track_type::TrackType,
    user::value_objects::id::UserId,
};

#[derive(Debug, Serialize)]
pub struct TrackPublishedEvent {
    id: String,
    user_id: UserId,
    room_id: RoomId,
    track_type: TrackType,
    created_at: DateTime<Utc>,
}

impl TrackPublishedEvent {
    pub fn new(
        id: String,
        user_id: UserId,
        room_id: RoomId,
        track_type: TrackType,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            user_id,
            room_id,
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
    room_id: RoomId,
    track_type: TrackType,
    created_at: DateTime<Utc>,
}

impl TrackUnpublishedEvent {
    pub fn new(
        id: String,
        user_id: UserId,
        room_id: RoomId,
        track_type: TrackType,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            user_id,
            room_id,
            track_type,
            created_at,
        }
    }

    pub fn room_id(&self) -> &RoomId {
        &self.room_id
    }
}
