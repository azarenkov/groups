use std::collections::HashSet;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use messaging_domain::channel::id::ChannelId;

#[derive(Debug, Clone)]
pub struct LiveParticipant {
    pub identity: String,
    pub session_id: String,
    pub camera_on: bool,
    pub screen_on: bool,
    pub joined_at: Option<DateTime<Utc>>,
}

#[derive(Debug, thiserror::Error)]
pub enum VoiceRoomInspectorError {
    #[error("Voice backend error: {0}")]
    Backend(String),
}

#[async_trait]
pub trait VoiceRoomInspector: Send + Sync {
    async fn live_rooms(&self) -> Result<HashSet<ChannelId>, VoiceRoomInspectorError>;
    async fn participants(&self, channel_id: &ChannelId) -> Result<Vec<LiveParticipant>, VoiceRoomInspectorError>;
}
