use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{
    room::value_objects::{
        id::RoomId, max_paticipants::MaxParticipants, name::RoomName, room_type::RoomType,
    },
    server::value_objects::id::ServerId,
    shared::value_objects::password_hash::PasswordHash,
    user::value_objects::id::UserId,
};

#[derive(Debug, Serialize)]
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
    #[allow(clippy::too_many_arguments)]
    pub fn new(
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
    ) -> Self {
        Self {
            id,
            server_id,
            owner_id,
            name,
            max_participants,
            room_type,
            password_hash,
            created_at,
            updated_at,
            last_join,
        }
    }

    pub fn create(
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
