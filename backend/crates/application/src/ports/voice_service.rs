use async_trait::async_trait;
use domain::{
    channel::{id::ChannelId, token::ChannelToken},
    server::id::ServerId,
    user::{id::UserId, name::UserName},
};

#[async_trait]
pub trait VoiceService {
    async fn create_token(
        &self,
        user_id: &UserId,
        username: &UserName,
        channel_id: &ChannelId,
    ) -> Result<ChannelToken, VoiceServiceError>;
    async fn create_voice_channel(
        &self,
        channel_id: &ChannelId,
        server_id: &ServerId,
    ) -> Result<(), VoiceServiceError>;
    fn client_url(&self) -> String;
}

#[derive(Debug, thiserror::Error)]
pub enum VoiceServiceError {
    #[error("Token creation error: {0}")]
    TokenCreation(String),
    #[error("Voice channel creation error: {0}")]
    VoiceChannelCreation(String),
}
