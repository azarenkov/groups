use chrono::{DateTime, Utc};

use crate::value_objects::{
    password_hash::PasswordHash,
    room::{id::RoomId, max_paticipants::MaxParticipants, name::RoomName},
    server::id::ServerId,
    user::id::UserId,
};

#[derive(Debug, PartialEq)]
pub enum RoomType {
    Public,
    Private,
}

pub struct Room {
    id: RoomId,
    server_id: ServerId,
    owner_id: UserId,
    name: RoomName,
    max_participants: MaxParticipants,
    room_type: RoomType,
    password_hash: Option<PasswordHash>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_join: DateTime<Utc>,
}

impl Room {
    pub fn new(
        server_id: ServerId,
        owner_id: UserId,
        name: RoomName,
        room_type: RoomType,
        password_hash: Option<PasswordHash>,
    ) -> Self {
        let now = Utc::now();

        let max_participants = MaxParticipants::default();

        Self {
            id: RoomId::generate(),
            server_id,
            owner_id,
            name,
            max_participants,
            room_type,
            password_hash,
            created_at: now,
            updated_at: now,
            last_join: now,
        }
    }

    pub fn id(&self) -> &RoomId {
        &self.id
    }

    pub fn owner_id(&self) -> &UserId {
        &self.owner_id
    }

    pub fn name(&self) -> &RoomName {
        &self.name
    }

    pub fn max_participants(&self) -> &MaxParticipants {
        &self.max_participants
    }

    pub fn room_type(&self) -> &RoomType {
        &self.room_type
    }

    pub fn password_hash(&self) -> Option<&PasswordHash> {
        self.password_hash.as_ref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn last_join(&self) -> DateTime<Utc> {
        self.last_join
    }

    pub fn server_id(&self) -> &ServerId {
        &self.server_id
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RoomError {
    #[error("Room is full")]
    RoomFull,

    #[error("Room isn't active")]
    RoomNotActive,

    #[error("User not found")]
    ParticipantNotFound,

    #[error("Insufficient ermissions")]
    InsufficientPermissions,
}
