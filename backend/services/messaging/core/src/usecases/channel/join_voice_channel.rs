use std::sync::Arc;

use domain::{
    channel::{ChannelError, id::ChannelId, repository::ChannelRepository},
    shared::{outbox::to_outbox::OutboxError, repository::error::RepositoryError},
    user::{id::UserId, name::UserName},
};

use crate::{
    dto::{
        requests::channel::join_voice_channel::JoinVoiceChannelRequest,
        responses::channel::join_voice_channel::JoinVoiceChannelResponse,
    },
    ports::{
        hasher::{HashError, Hasher},
        voice_service::{VoiceService, VoiceServiceError},
    },
};

#[derive(Debug, thiserror::Error)]
pub enum JoinVoiceChannelError {
    #[error("Channel not found")]
    ChannelNotFound,
    #[error("Password is required")]
    PasswordRequired,
    #[error("Invalid password")]
    InvalidPassword,
    #[error("User not found")]
    UserNotFound,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Voice service error: {0}")]
    VoiceService(#[from] VoiceServiceError),
    #[error("Hash error: {0}")]
    Hash(#[from] HashError),
    #[error("Channel domain error: {0}")]
    Domain(#[from] ChannelError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
    #[error("Voice not supported for this channel")]
    VoiceNotSupported,
}

#[derive(Debug)]
pub struct JoinVoiceChannel<T, U, R, H>
where
    T: VoiceService,
    U: ChannelRepository,
    R: domain::user_profile::UserProfileRepository,
    H: Hasher,
{
    voice_service: Arc<T>,
    channel_repository: Arc<U>,
    user_repository: Arc<R>,
    hasher: Arc<H>,
}

impl<T, U, R, H> JoinVoiceChannel<T, U, R, H>
where
    T: VoiceService,
    U: ChannelRepository,
    R: domain::user_profile::UserProfileRepository,
    H: Hasher,
{
    pub fn new(
        voice_service: Arc<T>,
        channel_repository: Arc<U>,
        user_repository: Arc<R>,
        hasher: Arc<H>,
    ) -> Self {
        Self {
            voice_service,
            channel_repository,
            user_repository,
            hasher,
        }
    }

    pub async fn execute(
        &self,
        request: JoinVoiceChannelRequest,
        user_id: UserId,
    ) -> Result<JoinVoiceChannelResponse, JoinVoiceChannelError> {
        let channel_id = ChannelId::new(request.channel_id);

        let channel = self
            .channel_repository
            .find_channel_by_id(&channel_id)
            .await?;

        if channel.server_id().uuid() != request.server_id {
            return Err(JoinVoiceChannelError::ChannelNotFound);
        }

        if !channel.kind().supports_voice() {
            return Err(JoinVoiceChannelError::VoiceNotSupported);
        }

        if let Some(expected_hash) = channel.password_hash() {
            let provided = request
                .password
                .ok_or(JoinVoiceChannelError::PasswordRequired)?;
            if !self.hasher.verify(&provided, expected_hash.as_str())? {
                return Err(JoinVoiceChannelError::InvalidPassword);
            }
        }

        let user_name = UserName::new(
            self.user_repository
                .get_name(&user_id)
                .await?
                .ok_or(JoinVoiceChannelError::UserNotFound)?,
        );

        self.voice_service
            .create_voice_channel(channel.id(), channel.server_id())
            .await?;

        let channel_token = self
            .voice_service
            .create_token(&user_id, &user_name, channel.id())
            .await?;

        let mute_record = channel
            .muted_participants_list()
            .find(|m| m.user_id == user_id);
        let response = JoinVoiceChannelResponse::new(
            channel_token,
            self.voice_service.client_url(),
            mute_record.is_some(),
            mute_record.map(|m| m.muted_by.uuid()),
        );
        Ok(response)
    }
}
