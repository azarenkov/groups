use std::{collections::HashSet, sync::Arc};

use async_trait::async_trait;
use infrastructure_common::voice::livekit_service::LiveKitService;
use messaging::ports::voice_room_inspector::{LiveParticipant, VoiceRoomInspector, VoiceRoomInspectorError};
use messaging_domain::channel::id::ChannelId;
use ports::voice_service::VoiceService;

pub struct LiveKitRoomInspector {
    service: Arc<LiveKitService>,
}

impl LiveKitRoomInspector {
    pub fn new(service: Arc<LiveKitService>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl VoiceRoomInspector for LiveKitRoomInspector {
    async fn live_rooms(&self) -> Result<HashSet<ChannelId>, VoiceRoomInspectorError> {
        let rooms = self.service.active_rooms().await.map_err(|e| VoiceRoomInspectorError::Backend(e.to_string()))?;
        Ok(rooms.into_iter().collect())
    }

    async fn participants(&self, channel_id: &ChannelId) -> Result<Vec<LiveParticipant>, VoiceRoomInspectorError> {
        let members = self.service.room_members(channel_id).await.map_err(|e| VoiceRoomInspectorError::Backend(e.to_string()))?;
        Ok(members
            .into_iter()
            .map(|m| LiveParticipant {
                identity: m.identity,
                session_id: m.session_id,
                camera_on: m.camera_on,
                screen_on: m.screen_on,
                joined_at: m.joined_at,
            })
            .collect())
    }
}
