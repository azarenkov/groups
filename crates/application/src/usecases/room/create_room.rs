use domain::{
    entites::room::{Room, RoomError},
    repositories::room::{RoomRepository, RoomRepositoryError},
    value_objects::{room::name::RoomName, user::id::UserId},
};
use validator::{Validate, ValidationErrors};

use crate::dto::room::create_room::CreateRoomRequest;

#[derive(Debug, thiserror::Error)]
pub enum CreateVoiceRoomError {
    #[error("Domain error: {0}")]
    Domain(#[from] RoomError),
    #[error("Room repository error: {0}")]
    Repository(#[from] RoomRepositoryError),
    #[error("Validation request error: {0}")]
    Validation(#[from] ValidationErrors),
}

pub struct CreateRoom<T>
where
    T: RoomRepository,
{
    room_repository: T,
}

impl<T> CreateRoom<T>
where
    T: RoomRepository,
{
    pub fn new(room_repository: T) -> Self {
        Self { room_repository }
    }

    async fn execute(
        &self,
        request: CreateRoomRequest,
        owner_id: UserId,
    ) -> Result<(), CreateVoiceRoomError> {
        request.validate()?;

        let room_name = RoomName::new(request.name);
        let room_type = request.room_type.into();
        let password_hash = todo!();

        let room = Room::new(owner_id, room_name, room_type, password_hash);
        self.room_repository.create_room(&room).await?;
        Ok(())
    }
}
