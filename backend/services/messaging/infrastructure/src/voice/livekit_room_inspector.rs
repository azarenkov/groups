use std::{collections::HashSet, time::Duration};

use async_trait::async_trait;
use infrastructure_common::config::livekit::LiveKitConfig;
use livekit_api::services::room::RoomClient;
use messaging::ports::voice_room_inspector::{LiveParticipant, VoiceRoomInspector, VoiceRoomInspectorError};
use messaging_domain::channel::id::ChannelId;
use tokio::time::timeout;
use uuid::Uuid;

const LK_SOURCE_CAMERA: i32 = 1;
const LK_SOURCE_SCREEN: i32 = 3;

pub struct LiveKitRoomInspector {
    client: RoomClient,
    request_timeout: Duration,
}

impl LiveKitRoomInspector {
    pub fn new(config: &LiveKitConfig, request_timeout: Duration) -> Self {
        Self {
            client: RoomClient::with_api_key(config.url(), config.api_key(), config.api_secret()),
            request_timeout,
        }
    }
}

#[async_trait]
impl VoiceRoomInspector for LiveKitRoomInspector {
    async fn live_rooms(&self) -> Result<HashSet<ChannelId>, VoiceRoomInspectorError> {
        match timeout(self.request_timeout, self.client.list_rooms(Vec::new())).await {
            Ok(Ok(rooms)) => Ok(rooms
                .into_iter()
                .filter_map(|r| Uuid::parse_str(&r.name).ok())
                .map(ChannelId::new)
                .collect()),
            Ok(Err(e)) => Err(VoiceRoomInspectorError::Backend(e.to_string())),
            Err(_) => Err(VoiceRoomInspectorError::Timeout),
        }
    }

    async fn participants(&self, channel_id: &ChannelId) -> Result<Vec<LiveParticipant>, VoiceRoomInspectorError> {
        let room = channel_id.uuid().to_string();
        match timeout(self.request_timeout, self.client.list_participants(&room)).await {
            Ok(Ok(participants)) => Ok(participants
                .into_iter()
                .map(|p| LiveParticipant {
                    camera_on: p.tracks.iter().any(|t| t.source == LK_SOURCE_CAMERA),
                    screen_on: p.tracks.iter().any(|t| t.source == LK_SOURCE_SCREEN),
                    identity: p.identity,
                    session_id: p.sid,
                })
                .collect()),
            Ok(Err(e)) => Err(VoiceRoomInspectorError::Backend(e.to_string())),
            Err(_) => Err(VoiceRoomInspectorError::Timeout),
        }
    }
}
