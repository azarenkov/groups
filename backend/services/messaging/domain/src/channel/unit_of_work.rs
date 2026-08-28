use domain::user::id::UserId;
use ports::repository::RepositoryError;

use crate::{channel::Channel, channel::events::ChannelEvent, channel::id::ChannelId, events::MessagingEvent};

#[async_trait::async_trait]
pub trait ChannelUnitOfWork: Send + Sync {
    async fn begin(&self) -> Result<Box<dyn ChannelTransaction>, RepositoryError>;
}

#[async_trait::async_trait]
pub trait ChannelTransaction: Send {
    async fn claim_inbox(&mut self, source: &str, external_id: &str) -> Result<bool, RepositoryError>;
    async fn load_channel_for_update(&mut self, channel_id: &ChannelId) -> Result<Channel, RepositoryError>;
    async fn save_voice_state(
        &mut self,
        channel: &mut Channel,
        events: &[ChannelEvent],
        request_id: Option<&str>,
    ) -> Result<(), RepositoryError>;
    async fn join_text(&mut self, channel_id: &ChannelId, user_id: &UserId) -> Result<bool, RepositoryError>;
    async fn save_events(&mut self, events: &[MessagingEvent], request_id: Option<&str>) -> Result<(), RepositoryError>;
    async fn commit(self: Box<Self>) -> Result<(), RepositoryError>;
}
