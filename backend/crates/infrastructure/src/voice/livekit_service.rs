use application::ports::voice_service::{VoiceService, VoiceServiceError};
use async_trait::async_trait;

use domain::{
    room::value_objects::{id::RoomId, token::RoomToken},
    server::value_objects::id::ServerId,
    user::value_objects::{id::UserId, name::UserName},
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
        room_id: &RoomId,
    ) -> Result<RoomToken, VoiceServiceError> {
        let token = AccessToken::with_api_key(self.config.api_key(), self.config.api_secret())
            .with_identity(&user_id.uuid().to_string())
            .with_name(username.as_str())
            .with_grants(access_token::VideoGrants {
                room_join: true,
                room: room_id.uuid().to_string(),
                ..Default::default()
            })
            .to_jwt()
            .map_err(|e| VoiceServiceError::VoiceRoomCreation(e.to_string()))?;

        Ok(RoomToken::new(token))
    }
    async fn create_voice_room(
        &self,
        room_id: &RoomId,
        server_id: &ServerId,
    ) -> Result<(), VoiceServiceError> {
        let room_service = RoomClient::new(self.config.url())
            .map_err(|e| VoiceServiceError::VoiceRoomCreation(e.to_string()))?;

        room_service
            .create_room(
                &room_id.uuid().to_string(),
                CreateRoomOptions {
                    metadata: server_id.uuid().to_string(),
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| VoiceServiceError::VoiceRoomCreation(e.to_string()))?;

        Ok(())
    }

    fn client_url(&self) -> String {
        self.config.client_url.clone()
    }
}
