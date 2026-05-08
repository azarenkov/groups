use async_trait::async_trait;
use futures::stream::BoxStream;
use uuid::Uuid;

#[async_trait]
pub trait EventConsumer<T> {
    async fn subscribe(&self, subject: &str) -> Result<BoxStream<'static, T>, EventConsumerError>;

    async fn subscribe_to_servers(
        &self,
        server_ids: Vec<Uuid>,
    ) -> Result<BoxStream<'static, T>, EventConsumerError>;
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
