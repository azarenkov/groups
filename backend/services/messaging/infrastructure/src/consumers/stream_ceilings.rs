use std::sync::Arc;

use async_trait::async_trait;
use domain::{stream::StreamCeiling, user::id::UserId};
use messaging_domain::{badge::BadgeRepository, channel::repository::ChannelRepository};
use ports::outbox_writer::OutboxWriter;
use ports::voice_service::{VoiceService, enforce_stream_ceiling, stream_limited_record};

#[async_trait]
pub trait TierChanged: Send + Sync {
    async fn tier_changed(&self, user_id: &UserId) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

pub struct ChannelStreamCeilings<B, C, V, O> {
    badges: Arc<B>,
    channels: Arc<C>,
    voice: Arc<V>,
    outbox: Arc<O>,
}

impl<B, C, V, O> ChannelStreamCeilings<B, C, V, O> {
    pub fn new(badges: Arc<B>, channels: Arc<C>, voice: Arc<V>, outbox: Arc<O>) -> Self {
        Self { badges, channels, voice, outbox }
    }
}

#[async_trait]
impl<B, C, V, O> TierChanged for ChannelStreamCeilings<B, C, V, O>
where
    B: BadgeRepository + Send + Sync,
    C: ChannelRepository + Send + Sync,
    V: VoiceService + Send + Sync,
    O: OutboxWriter + Send + Sync,
{
    async fn tier_changed(&self, user_id: &UserId) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let rooms = self.channels.list_voice_channel_ids_with_participant(user_id).await?;
        if rooms.is_empty() {
            return Ok(());
        }
        let ceiling = StreamCeiling::for_tier(&self.badges.tier_of(user_id, chrono::Utc::now()).await?);
        for room in rooms {
            let room = domain::channel::id::ChannelId::new(room.uuid());
            if enforce_stream_ceiling(&*self.voice, &room, user_id, ceiling).await? {
                self.outbox.save(&[stream_limited_record(user_id, &room, ceiling)]).await?;
            }
        }
        Ok(())
    }
}
