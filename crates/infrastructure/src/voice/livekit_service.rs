use application::ports::voice_service::{VoiceService, VoiceServiceError};
use async_trait::async_trait;
use domain::{
    entites::user::User,
    value_objects::room::{name::RoomName, token::RoomToken},
};
use livekit_api::{
    access_token::{self, AccessToken},
    services::room::{CreateRoomOptions, RoomClient},
};

use crate::config::livekit::LiveKitConfig;

pub struct LiveKitService<'a> {
    config: &'a LiveKitConfig,
}

impl<'a> LiveKitService<'a> {
    pub fn new(config: &'a LiveKitConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl<'a> VoiceService for LiveKitService<'a> {
    async fn create_token(
        &self,
        user: &User,
        room_name: &RoomName,
    ) -> Result<RoomToken, VoiceServiceError> {
        let token = AccessToken::with_api_key(self.config.api_key(), self.config.api_secret())
            .with_name(user.name().as_str())
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
