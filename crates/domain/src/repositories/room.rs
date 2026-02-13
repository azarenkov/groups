use async_trait::async_trait;

use crate::{
    entites::room::Room,
    repositories::error::RepositoryError,
    value_objects::room::{id::RoomId, name::RoomName},
};

#[async_trait]
pub trait RoomRepository {
    async fn create_room(&self, room: &Room) -> Result<(), RepositoryError>;
    async fn get_room_name_by_id(&self, room_id: &RoomId) -> Result<RoomName, RepositoryError>;
}
