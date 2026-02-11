use async_trait::async_trait;
use domain::{
    entites::user::User,
    value_objects::room::{name::RoomName, token::RoomToken},
};

#[async_trait]
pub trait VoiceService {
    async fn create_token(
        &self,
        user: &User,
        room_name: &RoomName,
    ) -> Result<RoomToken, VoiceServiceError>;
    async fn create_voice_room(&self, room_name: &RoomName) -> Result<(), VoiceServiceError>;
}

#[derive(Debug, thiserror::Error)]
pub enum VoiceServiceError {
    #[error("Token creation error: {0}")]
    TokenCreation(String),
    #[error("Voice room creation error: {0}")]
    VoiceRoomCreation(String),
}
