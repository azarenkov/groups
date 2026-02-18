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
        let room_id = RoomId::new(request.room_id);
        let server_id = ServerId::new(request.server_id);

        if !self.room_repository.room_exists(&room_id).await? {
            return Err(GenerateTokenError::Repository(RepositoryError::NotFound));
        }

        let room_token = self
            .voice_service
            .create_token(&user_id, &username, &room_id, &server_id)
            .await?;

        Ok(room_token)
    }
}
