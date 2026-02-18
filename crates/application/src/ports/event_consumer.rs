use async_trait::async_trait;
use domain::room::events::RoomEvent;
use futures::stream::BoxStream;

#[async_trait]
pub trait EventConsumer<T> {
    async fn consume_events_by_id(
        &self,
        id: &T,
    ) -> BoxStream<Result<RoomEvent, EventConsumerError>>;
}

#[derive(Debug, thiserror::Error)]
pub enum EventConsumerError {
    #[error("Derialization error: {0}")]
    Derialization(String),
    #[error("Publish error: {0}")]
    Consume(String),
    #[error("Ack error: {0}")]
    Ack(String),
}
