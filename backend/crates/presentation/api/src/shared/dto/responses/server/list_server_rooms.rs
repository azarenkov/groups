use chrono::{DateTime, Utc};
use domain::room::{Room, room_type::RoomType};
use serde::Serialize;
use uuid::Uuid;

use crate::shared::dto::pagination::PaginationMeta;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListServerRoomsResponse {
    pub rooms: Vec<RoomResponse>,
    pub pagination: PaginationMeta,
}

impl ListServerRoomsResponse {
    pub fn new(rooms: Vec<Room>, pagination: PaginationMeta) -> Self {
        Self {
            rooms: rooms.into_iter().map(RoomResponse::from).collect(),
            pagination,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomResponse {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub room_type: RoomTypeResponse,
    pub max_participants: u8,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RoomTypeResponse {
    Public,
    Private,
}

impl From<Room> for RoomResponse {
    fn from(value: Room) -> Self {
        Self {
            id: value.id().uuid(),
            name: value.name().as_str().to_owned(),
            owner_id: value.owner_id().uuid(),
            room_type: value.room_type().into(),
            max_participants: value.max_participants().value(),
            created_at: value.created_at(),
        }
    }
}

impl From<&RoomType> for RoomTypeResponse {
    fn from(value: &RoomType) -> Self {
        match value {
            RoomType::Public => Self::Public,
            RoomType::Private => Self::Private,
        }
    }
}
