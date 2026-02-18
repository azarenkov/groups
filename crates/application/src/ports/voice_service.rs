use async_trait::async_trait;
use domain::{
    room::value_objects::{id::RoomId, name::RoomName, token::RoomToken},
    server::value_objects::id::ServerId,
    user::value_objects::{id::UserId, name::UserName},
};

#[async_trait]
pub trait VoiceService {
    async fn create_token(
        &self,
        user_id: &UserId,
        username: &UserName,
        room_id: &RoomId,
        server_id: &ServerId,
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
