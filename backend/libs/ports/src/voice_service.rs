use async_trait::async_trait;
use domain::{channel::id::ChannelId, channel::token::ChannelToken, community::id::CommunityId, stream::StreamCeiling, user::id::UserId, user::name::UserName};

#[async_trait]
pub trait VoiceService {
    async fn create_token(
        &self,
        user_id: &UserId,
        username: &UserName,
        channel_id: &ChannelId,
        ceiling: StreamCeiling,
    ) -> Result<ChannelToken, VoiceServiceError>;
    async fn create_voice_channel(
        &self,
        channel_id: &ChannelId,
        community_id: &CommunityId,
    ) -> Result<(), VoiceServiceError>;
    async fn delete_voice_channel(
        &self,
        channel_id: &ChannelId,
    ) -> Result<(), VoiceServiceError>;
    async fn remove_participant(
        &self,
        channel_id: &ChannelId,
        user_id: &UserId,
    ) -> Result<(), VoiceServiceError>;
    async fn create_token_with_access(
        &self,
        user_id: &UserId,
        username: &UserName,
        channel_id: &ChannelId,
        access: ParticipantAccess,
        ceiling: StreamCeiling,
    ) -> Result<ChannelToken, VoiceServiceError>;
    async fn apply_participant_access(
        &self,
        channel_id: &ChannelId,
        user_id: &UserId,
        access: ParticipantAccess,
    ) -> Result<(), VoiceServiceError>;
    async fn set_stream_ceiling(
        &self,
        channel_id: &ChannelId,
        user_id: &UserId,
        ceiling: StreamCeiling,
    ) -> Result<(), VoiceServiceError>;
    async fn screen_shares(
        &self,
        channel_id: &ChannelId,
        user_id: &UserId,
    ) -> Result<Vec<PublishedVideo>, VoiceServiceError>;
    async fn mute_track(
        &self,
        channel_id: &ChannelId,
        user_id: &UserId,
        track_sid: &str,
    ) -> Result<(), VoiceServiceError>;
    async fn active_rooms(&self) -> Result<Vec<ChannelId>, VoiceServiceError>;
    async fn room_screen_shares(&self, channel_id: &ChannelId) -> Result<Vec<(UserId, PublishedVideo)>, VoiceServiceError>;
    fn client_url(&self) -> String;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedVideo {
    pub sid: String,
    pub width: u32,
    pub height: u32,
    pub bitrate: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParticipantAccess {
    pub can_publish_microphone: bool,
    pub can_subscribe: bool,
}

impl ParticipantAccess {
    pub const FULL: Self = Self {
        can_publish_microphone: true,
        can_subscribe: true,
    };
}

#[derive(Debug, thiserror::Error)]
pub enum VoiceServiceError {
    #[error("Token creation error: {0}")]
    TokenCreation(String),
    #[error("Voice channel creation error: {0}")]
    VoiceChannelCreation(String),
    #[error("Voice channel deletion error: {0}")]
    VoiceChannelDeletion(String),
    #[error("Participant removal error: {0}")]
    ParticipantRemoval(String),
    #[error("Participant update error: {0}")]
    ParticipantUpdate(String),
}

pub fn stream_limited_record(
    user_id: &UserId,
    room: &ChannelId,
    track_sid: &str,
    ceiling: StreamCeiling,
) -> crate::outbox::record::OutboxRecord {
    let occurred_at = chrono::Utc::now();
    crate::outbox::record::OutboxRecord {
        id: uuid::Uuid::new_v4(),
        aggregate_id: user_id.uuid(),
        aggregate_type: crate::outbox::record::AggregateType::User,
        parent_id: None,
        parent_type: None,
        event_type: "stream_limited".to_string(),
        payload: serde_json::json!({
            "user_id": user_id.uuid(),
            "room_id": room.uuid(),
            "track_sid": track_sid,
            "width": ceiling.width,
            "height": ceiling.height,
            "frame_rate": ceiling.frame_rate,
            "max_bitrate": ceiling.max_bitrate,
            "occurred_at": occurred_at,
        }),
        occurred_at,
        request_id: None,
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stream_limited_record_carries_the_ceiling_for_the_user() {
        let room = ChannelId::new(uuid::Uuid::new_v4());
        let user = UserId::new(uuid::Uuid::new_v4());
        let record = stream_limited_record(&user, &room, "TR_1", StreamCeiling::PRO);
        assert_eq!(record.event_type, "stream_limited");
        assert_eq!(record.aggregate_id, user.uuid());
        assert_eq!(record.payload["height"], 1440);
        assert_eq!(record.payload["track_sid"], "TR_1");
        assert_eq!(record.payload["room_id"], serde_json::json!(room.uuid()));
    }
}
