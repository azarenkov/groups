use async_trait::async_trait;

use crate::entites::room::Room;

#[async_trait]
pub trait RoomRepository {
    async fn create_room(&self, room: &Room) -> Result<(), RoomRepositoryError>;
}

#[derive(Debug, thiserror::Error)]
pub enum RoomRepositoryError {}
