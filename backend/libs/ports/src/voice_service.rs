use async_trait::async_trait;
use domain::{channel::id::ChannelId, channel::token::ChannelToken, server::id::ServerId, user::id::UserId, user::name::UserName};

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
    async fn delete_voice_channel(
        &self,
        channel_id: &ChannelId,
    ) -> Result<(), VoiceServiceError>;
    async fn remove_participant(
        &self,
        channel_id: &ChannelId,
        user_id: &UserId,
    ) -> Result<(), VoiceServiceError>;
    async fn create_token_without_microphone(
        &self,
        user_id: &UserId,
        username: &UserName,
        channel_id: &ChannelId,
    ) -> Result<ChannelToken, VoiceServiceError>;
    async fn mute_participant(
        &self,
        channel_id: &ChannelId,
        user_id: &UserId,
    ) -> Result<(), VoiceServiceError>;
    async fn unmute_participant(
        &self,
        channel_id: &ChannelId,
        user_id: &UserId,
    ) -> Result<(), VoiceServiceError>;
    fn client_url(&self) -> String;
}

#[derive(Debug, thiserror::Error)]
pub enum VoiceServiceError {
    #[error("Token creation error: {0}")]
    TokenCreation(String),
    #[error("Voice channel creation error: {0}")]
    VoiceChannelCreation(String),
    #[error("Voice channel deletion error: {0}")]
    VoiceChannelDeletion(String),
    #[error("Participant removal error: {0}")]
    ParticipantRemoval(String),
}
