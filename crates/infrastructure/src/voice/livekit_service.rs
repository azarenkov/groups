use application::ports::voice_service::{VoiceService, VoiceServiceError};
use async_trait::async_trait;
use domain::value_objects::{
    room::{name::RoomName, token::RoomToken},
    user::name::UserName,
};
use livekit_api::{
    access_token::{self, AccessToken},
    services::room::{CreateRoomOptions, RoomClient},
};

use crate::config::livekit::LiveKitConfig;

pub struct LiveKitService {
    config: LiveKitConfig,
}

impl<'a> LiveKitService {
    pub fn new(config: LiveKitConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl<'a> VoiceService for LiveKitService {
    async fn create_token(
        &self,
        username: &UserName,
        room_name: &RoomName,
    ) -> Result<RoomToken, VoiceServiceError> {
        let token = AccessToken::with_api_key(self.config.api_key(), self.config.api_secret())
            .with_name(username.as_str())
            .with_grants(access_token::VideoGrants {
                room_join: true,
                room: room_name.as_str().to_owned(),
                ..Default::default()
            })
            .to_jwt()
            .map_err(|e| VoiceServiceError::VoiceRoomCreation(e.to_string()))?;

        Ok(RoomToken::new(token))
    }
    async fn create_voice_room(&self, room_name: &RoomName) -> Result<(), VoiceServiceError> {
        let room_service = RoomClient::new(self.config.url())
            .map_err(|e| VoiceServiceError::VoiceRoomCreation(e.to_string()))?;

        room_service
            .create_room(room_name.as_str(), CreateRoomOptions::default())
            .await
            .map_err(|e| VoiceServiceError::VoiceRoomCreation(e.to_string()))?;

        Ok(())
    }
}
