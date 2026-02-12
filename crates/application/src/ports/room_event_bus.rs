use async_trait::async_trait;

use crate::dto::room::event::RoomEventNotification;

#[derive(Debug, thiserror::Error)]
pub enum RoomEventBusError {}

#[async_trait]
pub trait RoomEventBus: Send + Sync {
    async fn publish_event(&self, event: RoomEventNotification) -> Result<(), RoomEventBusError>;
}
