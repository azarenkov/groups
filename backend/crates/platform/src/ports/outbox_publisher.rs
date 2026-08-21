use async_trait::async_trait;
use domain::shared::outbox::outbox_record::OutboxRecord;

#[async_trait]
pub trait OutboxPublisher: Send + Sync {
    async fn publish(&self, record: &OutboxRecord) -> Result<(), OutboxPublisherError>;
}

#[derive(Debug, thiserror::Error)]
pub enum OutboxPublisherError {
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Publish error: {0}")]
    Publish(String),
    #[error("Ack error: {0}")]
    Ack(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
}
