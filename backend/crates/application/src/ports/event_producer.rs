use async_trait::async_trait;

#[async_trait]
pub trait EventProducer<T> {
    async fn publish(&self, event: &T) -> Result<(), EventProducerError>;
    async fn delete_events_by_id(&self, id: String) -> Result<(), EventProducerError>;
}

#[derive(Debug, thiserror::Error)]
pub enum EventProducerError {
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Publish error: {0}")]
    Publish(String),
    #[error("Ack error: {0}")]
    Ack(String),
}
