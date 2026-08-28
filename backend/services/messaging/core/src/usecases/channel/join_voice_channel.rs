use std::sync::Arc;

use messaging_domain::{channel::ChannelError, channel::id::ChannelId, channel::moderation::Publication, channel::repository::ChannelRepository};
use domain::{user::id::UserId, user::name::UserName};
use ports::repository::RepositoryError;

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
    #[error("Voice not supported for this channel")]
    VoiceNotSupported,
    #[error("Not a member of this server")]
    NotMember,
}

#[derive(Debug)]
pub struct JoinVoiceChannel<T, U, R, H, S>
where
    T: VoiceService,
    U: ChannelRepository,
    R: ports::user_profile::UserProfileRepository,
    H: Hasher,
    S: messaging_domain::server::repository::ServerRepository,
{
    voice_service: Arc<T>,
    channel_repository: Arc<U>,
    user_repository: Arc<R>,
    hasher: Arc<H>,
    server_repository: Arc<S>,
}

impl<T, U, R, H, S> JoinVoiceChannel<T, U, R, H, S>
where
    T: VoiceService,
    U: ChannelRepository,
    R: ports::user_profile::UserProfileRepository,
    H: Hasher,
    S: messaging_domain::server::repository::ServerRepository,
{
    pub fn new(
        voice_service: Arc<T>,
        channel_repository: Arc<U>,
        user_repository: Arc<R>,
        hasher: Arc<H>,
        server_repository: Arc<S>,
    ) -> Self {
        Self {
            voice_service,
            channel_repository,
            user_repository,
            hasher,
            server_repository,
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

        let server_id = messaging_domain::server::id::ServerId::new(request.server_id);
        if !self
            .server_repository
            .is_server_member(&server_id, &user_id)
            .await?
        {
            return Err(JoinVoiceChannelError::NotMember);
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

        let mute_record = channel.mute_record(&user_id);
        let channel_token = match channel.publication_for(&user_id) {
            Publication::Restricted => {
                self.voice_service
                    .create_token_without_microphone(&user_id, &user_name, channel.id())
                    .await?
            }
            Publication::Full => {
                self.voice_service
                    .create_token(&user_id, &user_name, channel.id())
                    .await?
            }
        };

        let response = JoinVoiceChannelResponse::new(
            channel_token,
            self.voice_service.client_url(),
            mute_record.is_some(),
            mute_record.map(|m| m.muted_by.uuid()),
            mute_record.is_some_and(|m| m.self_muted),
        );
        Ok(response)
    }
}
