use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{
    room::id::RoomId,
    server::id::ServerId,
    user::{id::UserId, name::UserName},
};

#[derive(Debug, Serialize)]
pub struct ParticipantJoinedEvent {
    id: String,
    user_id: UserId,
    user_name: Option<UserName>,
    room_id: RoomId,
    server_id: Option<ServerId>,
    joined_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl ParticipantJoinedEvent {
    pub fn new(
        id: String,
        user_id: UserId,
        user_name: Option<UserName>,
        room_id: RoomId,
        server_id: Option<ServerId>,
        joined_at: DateTime<Utc>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            user_id,
            user_name,
            room_id,
            server_id,
            joined_at,
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
pub struct ParticipantLeftEvent {
    id: String,
    user_id: UserId,
    user_name: Option<UserName>,
    room_id: RoomId,
    server_id: Option<ServerId>,
    joined_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl ParticipantLeftEvent {
    pub fn new(
        id: String,
        user_id: UserId,
        user_name: Option<UserName>,
        room_id: RoomId,
        server_id: Option<ServerId>,
        joined_at: DateTime<Utc>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            user_id,
            user_name,
            room_id,
            server_id,
            joined_at,
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
pub struct ParticipantConnectionAbortedEvent {
    id: String,
    user_id: UserId,
    user_name: Option<UserName>,
    room_id: RoomId,
    server_id: Option<ServerId>,
    joined_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl ParticipantConnectionAbortedEvent {
    pub fn new(
        id: String,
        user_id: UserId,
        user_name: Option<UserName>,
        room_id: RoomId,
        server_id: Option<ServerId>,
        joined_at: DateTime<Utc>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            user_id,
            user_name,
            room_id,
            server_id,
            joined_at,
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
