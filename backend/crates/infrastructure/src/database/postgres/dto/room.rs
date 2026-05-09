use chrono::{DateTime, Utc};
use domain::{
    room::{
        Room,
        id::RoomId, max_paticipants::MaxParticipants, name::RoomName,
    },
    server::id::ServerId,
    shared::value_objects::password_hash::PasswordHash,
    user::id::UserId,
};
use sqlx::{FromRow, types::Uuid};

use super::room_type::RoomTypeDto;

#[derive(Debug, FromRow)]
pub struct RoomDto {
    pub id: Uuid,
    pub server_id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub max_participants: i32,
    pub room_type: RoomTypeDto,
    pub password_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_join: DateTime<Utc>,
}

impl From<RoomDto> for Room {
    fn from(value: RoomDto) -> Self {
        Self::new(
            RoomId::new(value.id),
            ServerId::new(value.server_id),
            UserId::new(value.owner_id),
            RoomName::new(value.name),
            MaxParticipants::new(value.max_participants as u8),
            value.room_type.into(),
            value.password_hash.map(PasswordHash::new),
            value.created_at,
            value.updated_at,
            value.last_join,
        )
    }
}
