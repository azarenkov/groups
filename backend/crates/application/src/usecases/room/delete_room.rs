use std::sync::Arc;

use domain::{
    room::{repository::RoomRepository, id::RoomId},
    shared::repository::error::RepositoryError,
    user::id::UserId,
};

#[derive(Debug, thiserror::Error)]
pub enum DeleteRoomError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct DeleteRoom<T>
where
    T: RoomRepository,
{
    room_repository: Arc<T>,
}

impl<T> DeleteRoom<T>
where
    T: RoomRepository,
{
    pub fn new(room_repository: Arc<T>) -> Self {
        Self { room_repository }
    }

    pub async fn execute(
        &self,
        room_id: RoomId,
        owner_id: UserId,
    ) -> Result<bool, DeleteRoomError> {
        Ok(self
            .room_repository
            .delete_room_by_owner(&room_id, &owner_id)
            .await?)
    }
}
