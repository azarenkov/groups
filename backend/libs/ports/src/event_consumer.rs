use futures::future::BoxFuture;

use crate::outbox::record::PartitionSequence;
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
    pub extra_subjects: Vec<String>,
    pub ack_wait: Option<std::time::Duration>,
}

#[derive(Debug, Clone, Copy)]
pub enum StartPosition {
    New,
    All,
}

#[derive(Debug, Clone, Copy)]
pub enum Disposition {
    Ack,
    Retry(Option<std::time::Duration>),
}

pub type DispositionFn =
    Box<dyn FnOnce(Disposition) -> BoxFuture<'static, Result<(), EventConsumerError>> + Send>;

pub struct ConsumedMessage {
    pub payload: Vec<u8>,
    pub request_id: Option<String>,
    pub subject: String,
    pub sequence: Option<PartitionSequence>,
    pub event_id: Option<String>,
    pub pending: Option<u64>,
    ack_fn: DispositionFn,
}

impl ConsumedMessage {
    pub fn new(
        payload: Vec<u8>,
        request_id: Option<String>,
        subject: String,
        sequence: Option<PartitionSequence>,
        ack_fn: DispositionFn,
    ) -> Self {
        Self {
            payload,
            request_id,
            subject,
            sequence,
            event_id: None,
            pending: None,
            ack_fn,
        }
    }

    pub fn with_pending(mut self, pending: Option<u64>) -> Self {
        self.pending = pending;
        self
    }

    pub fn with_event_id(mut self, event_id: Option<String>) -> Self {
        self.event_id = event_id;
        self
    }

    pub async fn ack(self) -> Result<(), EventConsumerError> {
        (self.ack_fn)(Disposition::Ack).await
    }

    pub async fn retry_after(self, delay: std::time::Duration) -> Result<(), EventConsumerError> {
        (self.ack_fn)(Disposition::Retry(Some(delay))).await
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
