use async_trait::async_trait;
use domain::{
    room::{id::RoomId, token::RoomToken},
    server::id::ServerId,
    user::{id::UserId, name::UserName},
};

#[async_trait]
pub trait VoiceService {
    async fn create_token(
        &self,
        user_id: &UserId,
        username: &UserName,
        room_id: &RoomId,
    ) -> Result<RoomToken, VoiceServiceError>;
    async fn create_voice_room(
        &self,
        room_id: &RoomId,
        server_id: &ServerId,
    ) -> Result<(), VoiceServiceError>;
    fn client_url(&self) -> String;
}

#[derive(Debug, thiserror::Error)]
pub enum VoiceServiceError {
    #[error("Token creation error: {0}")]
    TokenCreation(String),
    #[error("Voice room creation error: {0}")]
    VoiceRoomCreation(String),
}
