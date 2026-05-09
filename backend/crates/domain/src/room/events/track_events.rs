use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{
    room::id::RoomId,
    server::id::ServerId,
    shared::value_objects::track_type::TrackType,
    user::{id::UserId, name::UserName},
};

#[derive(Debug, Serialize)]
pub struct TrackPublishedEvent {
    id: String,
    user_id: UserId,
    user_name: Option<UserName>,
    room_id: RoomId,
    server_id: Option<ServerId>,
    track_type: Option<TrackType>,
    created_at: DateTime<Utc>,
}

impl TrackPublishedEvent {
    pub fn new(
        id: String,
        user_id: UserId,
        user_name: Option<UserName>,
        room_id: RoomId,
        server_id: Option<ServerId>,
        track_type: Option<TrackType>,
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

    pub fn server_id(&self) -> &Option<ServerId> {
        &self.server_id
    }
}

#[derive(Debug, Serialize)]
pub struct TrackUnpublishedEvent {
    id: String,
    user_id: UserId,
    user_name: Option<UserName>,
    room_id: RoomId,
    server_id: Option<ServerId>,
    track_type: Option<TrackType>,
    created_at: DateTime<Utc>,
}

impl TrackUnpublishedEvent {
    pub fn new(
        id: String,
        user_id: UserId,
        user_name: Option<UserName>,
        room_id: RoomId,
        server_id: Option<ServerId>,
        track_type: Option<TrackType>,
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

    pub fn server_id(&self) -> &Option<ServerId> {
        &self.server_id
    }
}
