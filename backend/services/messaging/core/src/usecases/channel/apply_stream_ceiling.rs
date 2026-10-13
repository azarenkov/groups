use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use domain::{channel::id::ChannelId, plan::Tier, stream::StreamCeiling, user::id::UserId};
use messaging_domain::{badge::BadgeRepository, channel::repository::ChannelRepository};
use ports::repository::RepositoryError;

use crate::ports::voice_service::{VoiceService, VoiceServiceError};

const LIVE_CHANGE_WINDOW_HOURS: i64 = 1;

#[derive(Debug, thiserror::Error)]
pub enum ApplyStreamCeilingError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Voice service error: {0}")]
    Voice(#[from] VoiceServiceError),
}

#[async_trait]
pub trait PlanChanged: Send + Sync {
    async fn plan_changed(
        &self,
        user_id: &UserId,
        occurred_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<(), ApplyStreamCeilingError>;
}

#[async_trait]
pub trait ParticipantJoined: Send + Sync {
    async fn participant_joined(
        &self,
        room: &ChannelId,
        user_id: &UserId,
        now: DateTime<Utc>,
    ) -> Result<(), ApplyStreamCeilingError>;
}

pub struct ApplyStreamCeiling<B, C, V>
where
    B: BadgeRepository,
    C: ChannelRepository,
    V: VoiceService,
{
    badges: Arc<B>,
    channels: Arc<C>,
    voice: Arc<V>,
}

impl<B, C, V> ApplyStreamCeiling<B, C, V>
where
    B: BadgeRepository,
    C: ChannelRepository,
    V: VoiceService,
{
    pub fn new(badges: Arc<B>, channels: Arc<C>, voice: Arc<V>) -> Self {
        Self { badges, channels, voice }
    }
}

#[async_trait]
impl<B, C, V> PlanChanged for ApplyStreamCeiling<B, C, V>
where
    B: BadgeRepository + Send + Sync,
    C: ChannelRepository + Send + Sync,
    V: VoiceService + Send + Sync,
{
    async fn plan_changed(
        &self,
        user_id: &UserId,
        occurred_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<(), ApplyStreamCeilingError> {
        if now - occurred_at > Duration::hours(LIVE_CHANGE_WINDOW_HOURS) {
            return Ok(());
        }
        let rooms = self.channels.list_voice_channel_ids_with_participant(user_id).await?;
        if rooms.is_empty() {
            return Ok(());
        }
        let ceiling = StreamCeiling::for_tier(Tier::parse(&self.badges.tier_of(user_id, now).await?));
        for room in rooms {
            self.voice
                .set_stream_ceiling(&ChannelId::new(room.uuid()), user_id, ceiling)
                .await?;
        }
        Ok(())
    }
}

#[async_trait]
impl<B, C, V> ParticipantJoined for ApplyStreamCeiling<B, C, V>
where
    B: BadgeRepository + Send + Sync,
    C: ChannelRepository + Send + Sync,
    V: VoiceService + Send + Sync,
{
    async fn participant_joined(
        &self,
        room: &ChannelId,
        user_id: &UserId,
        now: DateTime<Utc>,
    ) -> Result<(), ApplyStreamCeilingError> {
        let ceiling = StreamCeiling::for_tier(Tier::parse(&self.badges.tier_of(user_id, now).await?));
        self.voice.set_stream_ceiling(room, user_id, ceiling).await?;
        Ok(())
    }
}
