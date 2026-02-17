use async_trait::async_trait;

#[async_trait]
pub trait EventConsumer<T>: Send + Sync {}

#[derive(Debug, thiserror::Error)]
pub enum EventConsumerError {}
