use async_trait::async_trait;

use crate::{entites::room::Room, repositories::error::RepositoryError};

#[async_trait]
pub trait RoomRepository {
    async fn create_room(&self, room: &Room) -> Result<(), RepositoryError>;
}
