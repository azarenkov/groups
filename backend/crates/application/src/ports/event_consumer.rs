use futures::future::BoxFuture;
use futures::stream::BoxStream;

pub trait EventConsumer: Send + Sync {
    fn subscribe(
        &self,
        opts: SubscribeOptions,
    ) -> BoxFuture<
        '_,
        Result<BoxStream<'static, Result<ConsumedMessage, EventConsumerError>>, EventConsumerError>,
    >;
}

#[derive(Debug)]
pub struct SubscribeOptions {
    pub consumer_name: String,
    pub subject: String,
    pub start_position: StartPosition,
    pub max_deliver: Option<u32>,
    pub dlq_subject: Option<String>,
}

#[derive(Debug)]
pub enum StartPosition {
    New,
    All,
}

pub struct ConsumedMessage {
    pub payload: Vec<u8>,
    pub request_id: Option<String>,
    ack_fn: Box<dyn FnOnce() -> BoxFuture<'static, Result<(), EventConsumerError>> + Send>,
}

impl ConsumedMessage {
    pub fn new(
        payload: Vec<u8>,
        request_id: Option<String>,
        ack_fn: Box<dyn FnOnce() -> BoxFuture<'static, Result<(), EventConsumerError>> + Send>,
    ) -> Self {
        Self {
            payload,
            request_id,
            ack_fn,
        }
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
