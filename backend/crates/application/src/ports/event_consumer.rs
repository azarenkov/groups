use futures::future::BoxFuture;
use futures::stream::BoxStream;

pub trait EventConsumer: Send + Sync {
    fn subscribe(
        &self,
        consumer_name: &str,
        subject: &str,
    ) -> BoxFuture<
        '_,
        Result<BoxStream<'static, Result<ConsumedMessage, EventConsumerError>>, EventConsumerError>,
    >;
}

pub struct ConsumedMessage {
    pub payload: Vec<u8>,
    ack_fn: Box<dyn FnOnce() -> BoxFuture<'static, Result<(), EventConsumerError>> + Send>,
}

impl ConsumedMessage {
    pub fn new(
        payload: Vec<u8>,
        ack_fn: Box<dyn FnOnce() -> BoxFuture<'static, Result<(), EventConsumerError>> + Send>,
    ) -> Self {
        Self { payload, ack_fn }
    }

    pub async fn ack(self) -> Result<(), EventConsumerError> {
        (self.ack_fn)().await
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EventConsumerError {
    #[error("Subscribe error: {0}")]
    Subscribe(String),
    #[error("Stream error: {0}")]
    Stream(String),
    #[error("Ack error: {0}")]
    Ack(String),
}
