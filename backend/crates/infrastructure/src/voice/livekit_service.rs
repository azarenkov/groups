use std::time::Duration;

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
use tokio::{net::TcpStream, time::timeout};

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

pub async fn is_livekit_reachable(url: &str) -> bool {
    let Some(host_port) = parse_host_port(url) else {
        return false;
    };
    matches!(
        timeout(Duration::from_secs(2), TcpStream::connect(host_port.as_str())).await,
        Ok(Ok(_))
    )
}

fn parse_host_port(url: &str) -> Option<String> {
    let without_scheme = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .or_else(|| url.strip_prefix("ws://"))
        .or_else(|| url.strip_prefix("wss://"))
        .unwrap_or(url);
    let host_port = without_scheme.split('/').next()?.split('?').next()?;
    if host_port.is_empty() {
        return None;
    }
    if host_port.contains(':') {
        Some(host_port.to_string())
    } else {
        let default_port = if url.starts_with("https://") || url.starts_with("wss://") {
            443
        } else {
            80
        };
        Some(format!("{host_port}:{default_port}"))
    }
}
