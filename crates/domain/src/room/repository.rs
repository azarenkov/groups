use async_trait::async_trait;

use crate::{
    room::{
        Room,
        value_objects::{id::RoomId, name::RoomName},
    },
    shared::repository::error::RepositoryError,
    user::value_objects::id::UserId,
};

#[async_trait]
pub trait RoomRepository {
    async fn create_room(&self, room: &Room) -> Result<(), RepositoryError>;
    async fn get_room_name_by_id(&self, room_id: &RoomId) -> Result<RoomName, RepositoryError>;
    async fn delete_room_by_owner(
        &self,
        room_id: &RoomId,
        owner_id: &UserId,
    ) -> Result<bool, RepositoryError>;
}
