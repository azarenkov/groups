use async_trait::async_trait;

use crate::{
    room::{
        Room,
        value_objects::{id::RoomId, name::RoomName},
    },
    server::value_objects::id::ServerId,
    shared::repository::error::RepositoryError,
    user::value_objects::id::UserId,
};

#[async_trait]
pub trait RoomRepository {
    async fn create_room(&self, room: &Room) -> Result<(), RepositoryError>;
    async fn get_room_name_by_id(&self, room_id: &RoomId) -> Result<RoomName, RepositoryError>;
    async fn get_room_by_id(&self, room_id: &RoomId) -> Result<Room, RepositoryError>;
    async fn room_exists(&self, room_id: &RoomId) -> Result<bool, RepositoryError>;
    async fn room_exists_with_server_id(
        &self,
        room_id: &RoomId,
        server_id: &ServerId,
    ) -> Result<bool, RepositoryError>;

    async fn delete_room_by_owner(
        &self,
        room_id: &RoomId,
        owner_id: &UserId,
    ) -> Result<bool, RepositoryError>;
}
