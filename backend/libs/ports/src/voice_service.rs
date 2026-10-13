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

pub async fn enforce_stream_ceiling<V: VoiceService + ?Sized>(
    voice: &V,
    room: &ChannelId,
    user_id: &UserId,
    ceiling: StreamCeiling,
) -> Result<bool, VoiceServiceError> {
    voice.set_stream_ceiling(room, user_id, ceiling).await?;
    let mut limited = false;
    for video in voice.screen_shares(room, user_id).await? {
        if !ceiling.allows(video.width, video.height, video.bitrate) {
            voice.mute_track(room, user_id, &video.sid).await?;
            limited = true;
        }
    }
    Ok(limited)
}

pub fn stream_limited_record(user_id: &UserId, room: &ChannelId, ceiling: StreamCeiling) -> crate::outbox::record::OutboxRecord {
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
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct FakeVoice {
        shares: Vec<PublishedVideo>,
        ceilings: Mutex<Vec<StreamCeiling>>,
        muted: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl VoiceService for FakeVoice {
        async fn create_token(&self, _: &UserId, _: &UserName, _: &ChannelId, _: StreamCeiling) -> Result<ChannelToken, VoiceServiceError> {
            unimplemented!()
        }
        async fn create_voice_channel(&self, _: &ChannelId, _: &CommunityId) -> Result<(), VoiceServiceError> {
            unimplemented!()
        }
        async fn delete_voice_channel(&self, _: &ChannelId) -> Result<(), VoiceServiceError> {
            unimplemented!()
        }
        async fn remove_participant(&self, _: &ChannelId, _: &UserId) -> Result<(), VoiceServiceError> {
            unimplemented!()
        }
        async fn create_token_with_access(
            &self,
            _: &UserId,
            _: &UserName,
            _: &ChannelId,
            _: ParticipantAccess,
            _: StreamCeiling,
        ) -> Result<ChannelToken, VoiceServiceError> {
            unimplemented!()
        }
        async fn apply_participant_access(&self, _: &ChannelId, _: &UserId, _: ParticipantAccess) -> Result<(), VoiceServiceError> {
            unimplemented!()
        }
        async fn set_stream_ceiling(&self, _: &ChannelId, _: &UserId, ceiling: StreamCeiling) -> Result<(), VoiceServiceError> {
            self.ceilings.lock().unwrap().push(ceiling);
            Ok(())
        }
        async fn screen_shares(&self, _: &ChannelId, _: &UserId) -> Result<Vec<PublishedVideo>, VoiceServiceError> {
            Ok(self.shares.clone())
        }
        async fn mute_track(&self, _: &ChannelId, _: &UserId, track_sid: &str) -> Result<(), VoiceServiceError> {
            self.muted.lock().unwrap().push(track_sid.to_string());
            Ok(())
        }
        async fn active_rooms(&self) -> Result<Vec<ChannelId>, VoiceServiceError> {
            unimplemented!()
        }
        async fn room_screen_shares(&self, _: &ChannelId) -> Result<Vec<(UserId, PublishedVideo)>, VoiceServiceError> {
            unimplemented!()
        }
        fn client_url(&self) -> String {
            String::new()
        }
    }

    fn share(sid: &str, width: u32, height: u32, bitrate: u64) -> PublishedVideo {
        PublishedVideo { sid: sid.into(), width, height, bitrate }
    }

    #[tokio::test]
    async fn mutes_only_the_shares_above_the_new_ceiling() {
        let voice = FakeVoice {
            shares: vec![share("ok", 1280, 720, 3_000_000), share("big", 1920, 1080, 6_000_000)],
            ..Default::default()
        };
        let room = ChannelId::new(uuid::Uuid::new_v4());
        let user = UserId::new(uuid::Uuid::new_v4());
        let limited = enforce_stream_ceiling(&voice, &room, &user, StreamCeiling::FREE).await.unwrap();
        assert!(limited);
        assert_eq!(*voice.muted.lock().unwrap(), vec!["big".to_string()]);
        assert_eq!(*voice.ceilings.lock().unwrap(), vec![StreamCeiling::FREE]);
    }

    #[tokio::test]
    async fn only_updates_the_ceiling_when_every_share_fits() {
        let voice = FakeVoice { shares: vec![share("ok", 1920, 1080, 6_000_000)], ..Default::default() };
        let room = ChannelId::new(uuid::Uuid::new_v4());
        let user = UserId::new(uuid::Uuid::new_v4());
        assert!(!enforce_stream_ceiling(&voice, &room, &user, StreamCeiling::PRO).await.unwrap());
        assert!(voice.muted.lock().unwrap().is_empty());
    }

    #[test]
    fn a_stream_limited_record_carries_the_ceiling_for_the_user() {
        let room = ChannelId::new(uuid::Uuid::new_v4());
        let user = UserId::new(uuid::Uuid::new_v4());
        let record = stream_limited_record(&user, &room, StreamCeiling::PRO);
        assert_eq!(record.event_type, "stream_limited");
        assert_eq!(record.aggregate_id, user.uuid());
        assert_eq!(record.payload["height"], 1440);
        assert_eq!(record.payload["room_id"], serde_json::json!(room.uuid()));
    }
}
