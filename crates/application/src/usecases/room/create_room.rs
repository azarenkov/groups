use std::sync::Arc;

use domain::{
    entites::room::{Room, RoomError, RoomType},
    repositories::{error::RepositoryError, room::RoomRepository},
    value_objects::{password_hash::PasswordHash, room::name::RoomName, user::id::UserId},
};

use crate::{
    dto::requests::room::create_room::CreateRoomRequest,
    ports::hasher::{HashError, Hasher},
};

#[derive(Debug, thiserror::Error)]
pub enum CreateRoomError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Hash error: {0}")]
    Hash(#[from] HashError),
    #[error("Password must be set")]
    PasswordRequired,
}

pub struct CreateRoom<T, H>
where
    T: RoomRepository,
    H: Hasher,
{
    room_repository: Arc<T>,
    hasher: Arc<H>,
}

impl<T, H> CreateRoom<T, H>
where
    T: RoomRepository,
    H: Hasher,
{
    pub fn new(room_repository: Arc<T>, hasher: Arc<H>) -> Self {
        Self {
            room_repository,
            hasher,
        }
    }

    pub async fn execute(
        &self,
        request: CreateRoomRequest,
        owner_id: UserId,
    ) -> Result<Room, CreateRoomError> {
        let room_name = RoomName::new(request.name);
        let room_type: RoomType = request.room_type.into();
        let password_hash = if room_type.eq(&RoomType::Private) {
            let password = request.password.ok_or(CreateRoomError::PasswordRequired)?;
            let hash = self.hasher.hash(&password)?;

            Some(PasswordHash::new(hash))
        } else {
            None
        };

        let room = Room::new(owner_id, room_name, room_type, password_hash);
        self.room_repository.create_room(&room).await?;
        Ok(room)
    }
}
