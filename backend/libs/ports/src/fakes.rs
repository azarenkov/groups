use std::collections::HashMap;
use std::sync::{Mutex, atomic::{AtomicBool, Ordering}};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::{
    channel::{id::ChannelId, token::ChannelToken},
    community::id::CommunityId,
    plan::Tier,
    stream::StreamCeiling,
    user::{id::UserId, name::UserName},
};
use uuid::Uuid;

use crate::plan_directory::PlanDirectory;
use crate::repository::RepositoryError;
use crate::voice_service::{ParticipantAccess, PublishedVideo, VoiceService, VoiceServiceError};

pub struct FakePlanDirectory {
    tiers: HashMap<Uuid, Tier>,
    fallback: Tier,
    caught_up: AtomicBool,
}

impl FakePlanDirectory {
    pub fn new(tiers: impl IntoIterator<Item = (Uuid, Tier)>) -> Self {
        Self { tiers: tiers.into_iter().collect(), fallback: Tier::Free, caught_up: AtomicBool::new(true) }
    }

    pub fn everyone(tier: Tier) -> Self {
        Self { tiers: HashMap::new(), fallback: tier, caught_up: AtomicBool::new(true) }
    }

    pub fn caught_up(self, caught_up: bool) -> Self {
        self.caught_up.store(caught_up, Ordering::SeqCst);
        self
    }
}

#[async_trait]
impl PlanDirectory for FakePlanDirectory {
    async fn apply_tier(&self, _: &UserId, _: Tier, _: Option<DateTime<Utc>>, _: DateTime<Utc>) -> Result<(), RepositoryError> {
        unimplemented!()
    }

    async fn tier_of(&self, user_id: &UserId, _: DateTime<Utc>) -> Result<Tier, RepositoryError> {
        Ok(self.tiers.get(&user_id.uuid()).copied().unwrap_or(self.fallback))
    }

    fn is_caught_up(&self) -> bool {
        self.caught_up.load(Ordering::SeqCst)
    }

    fn set_caught_up(&self, caught_up: bool) {
        self.caught_up.store(caught_up, Ordering::SeqCst);
    }
}

#[derive(Default)]
pub struct FakeVoiceService {
    rooms: Vec<(ChannelId, Vec<UserId>)>,
    videos: HashMap<ChannelId, Vec<(UserId, PublishedVideo)>>,
    unreachable: bool,
    pub ceilings: Mutex<Vec<(ChannelId, UserId, StreamCeiling)>>,
    pub muted: Mutex<Vec<String>>,
}

impl FakeVoiceService {
    pub fn room(mut self, room: ChannelId, participants: Vec<UserId>) -> Self {
        self.rooms.push((room, participants));
        self
    }

    pub fn videos(mut self, room: ChannelId, shares: Vec<(UserId, PublishedVideo)>) -> Self {
        self.videos.insert(room, shares);
        self
    }

    pub fn unreachable() -> Self {
        Self { unreachable: true, ..Self::default() }
    }
}

#[async_trait]
impl VoiceService for FakeVoiceService {
    async fn create_token(&self, _: &UserId, _: &UserName, _: &ChannelId) -> Result<ChannelToken, VoiceServiceError> {
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
    ) -> Result<ChannelToken, VoiceServiceError> {
        unimplemented!()
    }

    async fn apply_participant_access(&self, _: &ChannelId, _: &UserId, _: ParticipantAccess) -> Result<(), VoiceServiceError> {
        unimplemented!()
    }

    async fn set_stream_ceiling(&self, room: &ChannelId, user: &UserId, ceiling: StreamCeiling) -> Result<(), VoiceServiceError> {
        self.ceilings.lock().unwrap().push((*room, *user, ceiling));
        Ok(())
    }

    async fn mute_track(&self, _: &ChannelId, _: &UserId, track_sid: &str) -> Result<(), VoiceServiceError> {
        self.muted.lock().unwrap().push(track_sid.to_string());
        Ok(())
    }

    async fn active_rooms(&self) -> Result<Vec<ChannelId>, VoiceServiceError> {
        if self.unreachable {
            return Err(VoiceServiceError::ParticipantUpdate("down".into()));
        }
        Ok(self.rooms.iter().map(|(room, _)| *room).collect())
    }

    async fn room_participants(&self, room: &ChannelId) -> Result<Vec<UserId>, VoiceServiceError> {
        if self.unreachable {
            return Err(VoiceServiceError::ParticipantUpdate("down".into()));
        }
        Ok(self.rooms.iter().find(|(id, _)| id == room).map(|(_, users)| users.clone()).unwrap_or_default())
    }

    async fn room_videos(&self, room: &ChannelId) -> Result<Vec<(UserId, PublishedVideo)>, VoiceServiceError> {
        Ok(self.videos.get(room).cloned().unwrap_or_default())
    }

    fn client_url(&self) -> String {
        String::new()
    }
}
