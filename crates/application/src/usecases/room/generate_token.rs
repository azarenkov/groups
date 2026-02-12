use domain::{
    repositories::{error::RepositoryError, user::UserRepository},
    value_objects::{
        room::{name::RoomName, token::RoomToken},
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

pub struct GenerateToken<T, R>
where
    T: VoiceService,
    R: UserRepository,
{
    voice_service: T,
    user_repository: R,
}

impl<T, R> GenerateToken<T, R>
where
    T: VoiceService,
    R: UserRepository,
{
    pub fn new(voice_service: T, user_repository: R) -> Self {
        Self {
            voice_service,
            user_repository,
        }
    }

    pub async fn execute(
        &self,
        request: GenerateTokenRequest,
        user_id: UserId,
    ) -> Result<RoomToken, GenerateTokenError> {
        let username = self.user_repository.get_username_by_id(&user_id).await?;

        let room_name = RoomName::new(request.room_name);

        let room_token = self
            .voice_service
            .create_token(&username, &room_name)
            .await?;

        Ok(room_token)
    }
}
