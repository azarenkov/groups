use async_trait::async_trait;
use domain::events::room::RoomEvent;

#[derive(Debug, thiserror::Error)]
pub enum RoomEventBusError {}

#[async_trait]
pub trait RoomEventBus: Send + Sync {
    async fn publish_event(&self, event: RoomEvent) -> Result<(), RoomEventBusError>;
}
