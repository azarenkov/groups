use std::sync::Arc;

use domain::{
    room::{
        repository::RoomRepository,
        value_objects::{id::RoomId, token::RoomToken},
    },
    server::value_objects::id::ServerId,
    shared::repository::error::RepositoryError,
    user::{repository::UserRepository, value_objects::id::UserId},
};

use crate::{
    dto::requests::room::join_room::JoinRoomRequest,
    ports::voice_service::{VoiceService, VoiceServiceError},
};

#[derive(Debug, thiserror::Error)]
pub enum JoinRoomError {
    #[error("Voice service error: {0}")]
    VoiceService(#[from] VoiceServiceError),
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

#[derive(Debug)]
pub struct JoinRoom<T, U, R>
where
    T: VoiceService,
    U: RoomRepository,
    R: UserRepository,
{
    voice_service: Arc<T>,
    room_repository: Arc<U>,
    user_repository: Arc<R>,
}

impl<T, U, R> JoinRoom<T, U, R>
where
    T: VoiceService,
    U: RoomRepository,
    R: UserRepository,
{
    pub async fn execute(
        &self,
        request: JoinRoomRequest,
        user_id: UserId,
    ) -> Result<RoomToken, JoinRoomError> {
        let room_id = RoomId::new(request.room_id);
        let server_id = ServerId::new(request.server_id);
        let username = self.user_repository.get_username_by_id(&user_id).await?;
        if !self
            .room_repository
            .room_exists_with_server_id(&room_id, &server_id)
            .await?
        {
            return Err(JoinRoomError::Repository(RepositoryError::NotFound));
        }

        self.voice_service
            .create_voice_room(&room_id, &server_id)
            .await?;

        let room_token = self
            .voice_service
            .create_token(&user_id, &username, &room_id)
            .await?;
        Ok(room_token)
    }
}
