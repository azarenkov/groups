use std::sync::Mutex;

use async_trait::async_trait;
use domain::{
    channel::{id::ChannelId, token::ChannelToken},
    community::id::CommunityId,
    stream::StreamCeiling,
    user::{id::UserId, name::UserName},
};

use crate::voice_service::{ParticipantAccess, VoiceService, VoiceServiceError};

#[derive(Default)]
pub struct FakeVoiceService {
    rooms: Vec<(ChannelId, Vec<UserId>)>,
    unreachable: bool,
    pub ceilings: Mutex<Vec<(ChannelId, UserId, StreamCeiling)>>,
}

impl FakeVoiceService {
    pub fn room(mut self, room: ChannelId, participants: Vec<UserId>) -> Self {
        self.rooms.push((room, participants));
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

    fn client_url(&self) -> String {
        String::new()
    }
}
