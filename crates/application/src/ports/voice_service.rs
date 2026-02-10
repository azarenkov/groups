use async_trait::async_trait;
use domain::{entites::user::User, value_objects::room_token::RoomToken};

#[async_trait]
pub trait VoiceService {
    async fn create_token(&self, user: &User) -> Result<RoomToken, VoiceServiceError>;
    async fn create_voice_room(&self) -> Result<(), VoiceServiceError>;
}

#[derive(Debug, thiserror::Error)]
pub enum VoiceServiceError {
    #[error("Token error: {0}")]
    Token(String),
    #[error("Voice room error: {0}")]
    VoiceRoom(String),
}
