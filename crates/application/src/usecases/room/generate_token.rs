use std::sync::Arc;

use domain::{
    repositories::{error::RepositoryError, room::RoomRepository, user::UserRepository},
    value_objects::{
        room::{id::RoomId, name::RoomName, token::RoomToken},
        user::id::UserId,
    },
};

use crate::{
    dto::requests::room::generate_token::GenerateTokenRequest,
    ports::voice_service::{VoiceService, VoiceServiceError},
};

#[derive(Debug, thiserror::Error)]
pub enum GenerateTokenError {
    #[error("Voice service error: {0}")]
    VoiceService(#[from] VoiceServiceError),
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct GenerateToken<T, R, U>
where
    T: VoiceService,
    R: UserRepository,
    U: RoomRepository,
{
    voice_service: Arc<T>,
    user_repository: Arc<R>,
    room_repository: Arc<U>,
}

impl<T, R, U> GenerateToken<T, R, U>
where
    T: VoiceService,
    R: UserRepository,
    U: RoomRepository,
{
    pub fn new(voice_service: Arc<T>, user_repository: Arc<R>, room_repository: Arc<U>) -> Self {
        Self {
            voice_service,
            user_repository,
            room_repository,
        }
    }

    pub async fn execute(
        &self,
        request: GenerateTokenRequest,
        user_id: UserId,
    ) -> Result<RoomToken, GenerateTokenError> {
        let username = self.user_repository.get_username_by_id(&user_id).await?;

        let room_name = self
            .room_repository
            .get_room_name_by_id(&RoomId::new(request.room_id))
            .await?;

        let room_token = self
            .voice_service
            .create_token(&user_id, &username, &room_name)
            .await?;

        Ok(room_token)
    }
}
