use application::ports::voice_service::{VoiceService, VoiceServiceError};
use async_trait::async_trait;
use domain::{entites::user::User, value_objects::room_token::RoomToken};
use livekit::{Room, RoomOptions};
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
    async fn create_token(&self, user: &User) -> Result<RoomToken, VoiceServiceError> {
        let token = AccessToken::with_api_key(self.config.api_key(), self.config.api_secret())
            .with_name(user.nickname())
            .with_grants(access_token::VideoGrants {
                room_join: true,
                room: "my-room".to_string(),
                ..Default::default()
            })
            .to_jwt()
            .map_err(|e| VoiceServiceError::Token(e.to_string()))?;

        Ok(RoomToken::new(token))
    }
    async fn create_voice_room(&self) -> Result<(), VoiceServiceError> {
        let room_service = RoomClient::new(self.config.url()).unwrap();
        todo!();
        room_service
            .create_room("my_room", CreateRoomOptions::default())
            .await
            .map_err(|e| VoiceServiceError::VoiceRoom(e.to_string()))?;

        Ok(())
    }
}
