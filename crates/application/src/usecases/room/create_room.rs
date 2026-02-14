use std::sync::Arc;

use domain::{
    entites::room::{Room, RoomType},
    repositories::{error::RepositoryError, room::RoomRepository, server::ServerRepository},
    value_objects::{
        password_hash::PasswordHash, room::name::RoomName, server::id::ServerId, user::id::UserId,
    },
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
    #[error("Invalid data")]
    InvalidData,
}

pub struct CreateRoom<T, H, S>
where
    T: RoomRepository,
    H: Hasher,
    S: ServerRepository,
{
    room_repository: Arc<T>,
    hasher: Arc<H>,
    server_repository: Arc<S>,
}

impl<T, H, S> CreateRoom<T, H, S>
where
    T: RoomRepository,
    H: Hasher,
    S: ServerRepository,
{
    pub fn new(room_repository: Arc<T>, hasher: Arc<H>, server_repository: Arc<S>) -> Self {
        Self {
            room_repository,
            hasher,
            server_repository,
        }
    }

    pub async fn execute(
        &self,
        request: CreateRoomRequest,
        owner_id: UserId,
    ) -> Result<Room, CreateRoomError> {
        let server_id = ServerId::new(request.server_id);

        if !self.server_repository.server_exists(&server_id).await? {
            return Err(CreateRoomError::InvalidData);
        }

        let room_name = RoomName::new(request.name);
        let room_type: RoomType = request.room_type.into();
        let password_hash = if room_type.eq(&RoomType::Private) {
            let password = request.password.ok_or(CreateRoomError::PasswordRequired)?;
            let hash = self.hasher.hash(&password)?;

            Some(PasswordHash::new(hash))
        } else {
            None
        };

        let room = Room::new(server_id, owner_id, room_name, room_type, password_hash);
        self.room_repository.create_room(&room).await?;
        Ok(room)
    }
}
