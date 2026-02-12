use async_trait::async_trait;

use crate::{
    entites::room::Room,
    value_objects::user::{id::UserId, name::UserName},
};

#[async_trait]
pub trait RoomRepository {
    async fn create_room(&self, room: &Room) -> Result<(), RoomRepositoryError>;
    async fn get_username_by_id(&self, user_id: &UserId) -> Result<UserName, RoomRepositoryError>;
}

#[derive(Debug, thiserror::Error)]
pub enum RoomRepositoryError {}
