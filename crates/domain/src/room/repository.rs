use async_trait::async_trait;

use crate::{
    room::{
        Room,
        value_objects::{id::RoomId, name::RoomName},
    },
    shared::repository::error::RepositoryError,
};

#[async_trait]
pub trait RoomRepository {
    async fn create_room(&self, room: &Room) -> Result<(), RepositoryError>;
    async fn get_room_name_by_id(&self, room_id: &RoomId) -> Result<RoomName, RepositoryError>;
}
