use std::sync::Arc;

use domain::{
    channel::{repository::ChannelRepository, id::ChannelId},
    server::id::ServerId,
    shared::repository::error::RepositoryError,
    user::{repository::UserRepository, id::UserId},
};

use crate::{
    dto::{
        requests::channel::join_channel::JoinChannelRequest, responses::channel::join_channel::JoinChannelResponse,
    },
    ports::voice_service::{VoiceService, VoiceServiceError},
};

#[derive(Debug, thiserror::Error)]
pub enum JoinChannelError {
    #[error("Voice service error: {0}")]
    VoiceService(#[from] VoiceServiceError),
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

#[derive(Debug)]
pub struct JoinChannel<T, U, R>
where
    T: VoiceService,
    U: ChannelRepository,
    R: UserRepository,
{
    voice_service: Arc<T>,
    channel_repository: Arc<U>,
    user_repository: Arc<R>,
}

impl<T, U, R> JoinChannel<T, U, R>
where
    T: VoiceService,
    U: ChannelRepository,
    R: UserRepository,
{
    pub fn new(voice_service: Arc<T>, channel_repository: Arc<U>, user_repository: Arc<R>) -> Self {
        Self {
            voice_service,
            channel_repository,
            user_repository,
        }
    }

    pub async fn execute(
        &self,
        request: JoinChannelRequest,
        user_id: UserId,
    ) -> Result<JoinChannelResponse, JoinChannelError> {
        let channel_id = ChannelId::new(request.channel_id);
        let server_id = ServerId::new(request.server_id);
        let username = self.user_repository.get_username_by_id(&user_id).await?;
        if !self
            .channel_repository
            .channel_exists_with_server_id(&channel_id, &server_id)
            .await?
        {
            return Err(JoinChannelError::Repository(RepositoryError::NotFound));
        }

        self.voice_service
            .create_voice_channel(&channel_id, &server_id)
            .await?;

        let channel_token = self
            .voice_service
            .create_token(&user_id, &username, &channel_id)
            .await?;

        let client_url = self.voice_service.client_url();

        let response = JoinChannelResponse::new(channel_token, client_url);
        Ok(response)
    }
}
