use application::ports::voice_service::{VoiceService, VoiceServiceError};
use async_trait::async_trait;

use domain::{
    channel::{id::ChannelId, token::ChannelToken},
    server::id::ServerId,
    user::{id::UserId, name::UserName},
};
use livekit_api::{
    access_token::{self, AccessToken},
    services::room::{CreateRoomOptions, RoomClient},
};

use crate::config::livekit::LiveKitConfig;

pub struct LiveKitService {
    config: LiveKitConfig,
}

impl LiveKitService {
    pub fn new(config: LiveKitConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl VoiceService for LiveKitService {
    async fn create_token(
        &self,
        user_id: &UserId,
        username: &UserName,
        channel_id: &ChannelId,
    ) -> Result<ChannelToken, VoiceServiceError> {
        let token = AccessToken::with_api_key(self.config.api_key(), self.config.api_secret())
            .with_identity(&user_id.uuid().to_string())
            .with_name(username.as_str())
            .with_grants(access_token::VideoGrants {
                room_join: true,
                room: channel_id.uuid().to_string(),
                ..Default::default()
            })
            .to_jwt()
            .map_err(|e| VoiceServiceError::VoiceChannelCreation(e.to_string()))?;

        Ok(ChannelToken::new(token))
    }

    async fn create_voice_channel(
        &self,
        channel_id: &ChannelId,
        server_id: &ServerId,
    ) -> Result<(), VoiceServiceError> {
        let livekit_room = RoomClient::new(self.config.url())
            .map_err(|e| VoiceServiceError::VoiceChannelCreation(e.to_string()))?;

        livekit_room
            .create_room(
                &channel_id.uuid().to_string(),
                CreateRoomOptions {
                    metadata: server_id.uuid().to_string(),
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| VoiceServiceError::VoiceChannelCreation(e.to_string()))?;

        Ok(())
    }

    fn client_url(&self) -> String {
        self.config.client_url.clone()
    }
}
