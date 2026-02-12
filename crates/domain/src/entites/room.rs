use chrono::{DateTime, Utc};

use crate::value_objects::{
    password_hash::PasswordHash,
    room::{id::RoomId, max_paticipants::MaxParticipants, name::RoomName},
    user::id::UserId,
};

#[derive(Debug, PartialEq)]
pub enum RoomType {
    Public,
    Private,
}

pub struct Room {
    id: RoomId,
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
        owner_id: UserId,
        name: RoomName,
        room_type: RoomType,
        password_hash: Option<PasswordHash>,
    ) -> Self {
        let now = Utc::now();

        let max_participants = MaxParticipants::default();

        Self {
            id: RoomId::generate(),
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
