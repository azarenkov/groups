use std::{future::Future, sync::Arc, time::Duration};

use async_nats::jetstream::Context;
use futures::StreamExt;
use ports::event_consumer::{ConsumedMessage, EventConsumer, StartPosition, SubscribeOptions};
use ports::outbox::record::PartitionSequence;
use tracing::Instrument;

use crate::health::Heartbeat;
use crate::messaging::nats::consumer::NatsEventConsumer;

pub type HandlerError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Delivery {
    pub max_deliver: u32,
    pub retry_delay: Duration,
    pub ack_wait: Option<Duration>,
}

impl Delivery {
    pub const DEFAULT: Self = Self { max_deliver: 20, retry_delay: Duration::from_secs(5), ack_wait: None };

    pub const fn new(max_deliver: u32, retry_delay: Duration) -> Self {
        Self { max_deliver, retry_delay, ack_wait: None }
    }

    pub const fn with_ack_wait(mut self, ack_wait: Duration) -> Self {
        self.ack_wait = Some(ack_wait);
        self
    }
}

impl Default for Delivery {
    fn default() -> Self {
        Self::DEFAULT
    }
}

pub const RESUBSCRIBE_DELAY: Duration = Duration::from_secs(5);

pub fn subject_prefix(raw: &str) -> String {
    raw.trim_end_matches('.').to_string()
}

pub fn dlq_subject(prefix: &str, consumer_name: &str) -> String {
    format!("{}.dlq.{consumer_name}", subject_prefix(prefix))
}

#[derive(Debug, Clone)]
pub struct ConsumerSpec {
    pub stream: String,
    pub consumer_name: String,
    pub subjects: Vec<String>,
    pub dlq_subject: String,
    pub start_position: StartPosition,
    pub delivery: Delivery,
}

impl ConsumerSpec {
    pub fn new(prefix: &str, stream: impl Into<String>, consumer_name: impl Into<String>, subjects: Vec<String>) -> Self {
        let consumer_name = consumer_name.into();
        Self {
            stream: stream.into(),
            dlq_subject: dlq_subject(prefix, &consumer_name),
            consumer_name,
            subjects,
            start_position: StartPosition::All,
            delivery: Delivery::DEFAULT,
        }
    }

    pub fn starting(mut self, start_position: StartPosition) -> Self {
        self.start_position = start_position;
        self
    }

    pub fn delivered(mut self, delivery: Delivery) -> Self {
        self.delivery = delivery;
        self
    }

    fn subscribe_options(&self) -> SubscribeOptions {
        let mut subjects = self.subjects.clone();
        let subject = if subjects.is_empty() { String::new() } else { subjects.remove(0) };
        SubscribeOptions {
            consumer_name: self.consumer_name.clone(),
            subject,
            start_position: self.start_position,
            max_deliver: Some(self.delivery.max_deliver),
            dlq_subject: Some(self.dlq_subject.clone()),
            extra_subjects: subjects,
            ack_wait: self.delivery.ack_wait,
        }
    }
}

pub struct Event {
    pub subject: String,
    pub payload: Vec<u8>,
    pub request_id: Option<String>,
    pub event_id: Option<String>,
    pub sequence: Option<PartitionSequence>,
    pub pending: Option<u64>,
}

impl Event {
    fn from_message(message: &ConsumedMessage) -> Self {
        Self {
            subject: message.subject.clone(),
            payload: message.payload.clone(),
            request_id: message.request_id.clone(),
            event_id: message.event_id.clone(),
            sequence: message.sequence.clone(),
            pending: message.pending,
        }
    }
}

pub type IdleHook = Arc<dyn Fn() -> futures::future::BoxFuture<'static, ()> + Send + Sync>;

#[derive(Default, Clone)]
pub struct Lifecycle {
    pub shutdown: Option<tokio::sync::watch::Receiver<bool>>,
    pub heartbeat: Option<Heartbeat>,
    pub on_subscribed: Option<Arc<dyn Fn() + Send + Sync>>,
    pub on_disconnected: Option<Arc<dyn Fn() + Send + Sync>>,
    pub idle: Option<(Duration, IdleHook)>,
}

impl Lifecycle {
    pub fn with_shutdown(mut self, shutdown: tokio::sync::watch::Receiver<bool>, heartbeat: Heartbeat) -> Self {
        self.shutdown = Some(shutdown);
        self.heartbeat = Some(heartbeat);
        self
    }

    pub fn on_subscribed(mut self, hook: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_subscribed = Some(Arc::new(hook));
        self
    }

    pub fn on_disconnected(mut self, hook: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_disconnected = Some(Arc::new(hook));
        self
    }

    pub fn when_idle(mut self, after: Duration, hook: IdleHook) -> Self {
        self.idle = Some((after, hook));
        self
    }

    fn beat(&self) {
        if let Some(heartbeat) = &self.heartbeat {
            heartbeat.beat();
        }
    }

    fn stopped(&self) -> bool {
        self.shutdown.as_ref().is_some_and(|shutdown| *shutdown.borrow())
    }
}

enum Next {
    Message(Result<ConsumedMessage, ports::event_consumer::EventConsumerError>),
    Idle,
    Ended,
    Stop,
}

pub async fn run_consumer<H, Fut>(jetstream: Context, spec: ConsumerSpec, lifecycle: Lifecycle, handler: H)
where
    H: Fn(Event) -> Fut,
    Fut: Future<Output = Result<(), HandlerError>>,
{
    let consumer = NatsEventConsumer::new(jetstream, spec.stream.clone());
    let mut shutdown = lifecycle.shutdown.clone();
    loop {
        lifecycle.beat();
        if lifecycle.stopped() {
            return;
        }
        match consumer.subscribe(spec.subscribe_options()).await {
            Ok(mut messages) => {
                tracing::info!(
                    stream = %spec.stream,
                    consumer = %spec.consumer_name,
                    dlq = %spec.dlq_subject,
                    max_deliver = spec.delivery.max_deliver,
                    "Consumer subscribed",
                );
                if let Some(hook) = &lifecycle.on_subscribed {
                    hook();
                }
                loop {
                    lifecycle.beat();
                    let next = match (&mut shutdown, &lifecycle.idle) {
                        (Some(shutdown), _) => tokio::select! {
                            _ = shutdown.changed() => Next::Stop,
                            msg = messages.next() => msg.map_or(Next::Ended, Next::Message),
                        },
                        (None, Some((after, _))) => match tokio::time::timeout(*after, messages.next()).await {
                            Ok(Some(msg)) => Next::Message(msg),
                            Ok(None) => Next::Ended,
                            Err(_) => Next::Idle,
                        },
                        (None, None) => messages.next().await.map_or(Next::Ended, Next::Message),
                    };
                    match next {
                        Next::Stop => return,
                        Next::Idle => {
                            if let Some((_, hook)) = &lifecycle.idle {
                                hook().await;
                            }
                        }
                        Next::Ended => {
                            tracing::warn!(stream = %spec.stream, consumer = %spec.consumer_name, "Consumer stream ended");
                            break;
                        }
                        Next::Message(Err(e)) => {
                            tracing::warn!(error = %e, stream = %spec.stream, consumer = %spec.consumer_name, "Consumer stream error");
                            break;
                        }
                        Next::Message(Ok(message)) => {
                            let event = Event::from_message(&message);
                            let span = tracing::info_span!("event", request_id = ?event.request_id, subject = %event.subject);
                            let outcome = handler(event).instrument(span.clone()).await;
                            async {
                                match outcome {
                                    Ok(()) => {
                                        if let Err(e) = message.ack().await {
                                            tracing::warn!(error = %e, "Ack failed");
                                        }
                                    }
                                    Err(e) => {
                                        tracing::warn!(error = %e, "Handler failed; scheduling retry");
                                        if let Err(e) = message.retry_after(spec.delivery.retry_delay).await {
                                            tracing::warn!(error = %e, "Nak failed");
                                        }
                                    }
                                }
                            }
                            .instrument(span)
                            .await;
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, stream = %spec.stream, consumer = %spec.consumer_name, "Subscribe failed; retrying");
            }
        }
        if let Some(hook) = &lifecycle.on_disconnected {
            hook();
        }
        match (&mut shutdown, &lifecycle.heartbeat) {
            (Some(shutdown), Some(heartbeat)) => tokio::select! {
                _ = shutdown.changed() => return,
                _ = heartbeat.keep_alive_for(RESUBSCRIBE_DELAY) => {}
            },
            _ => tokio::time::sleep(RESUBSCRIBE_DELAY).await,
        }
    }
}

pub fn spawn_consumer<H, Fut>(jetstream: Context, spec: ConsumerSpec, lifecycle: Lifecycle, handler: H)
where
    H: Fn(Event) -> Fut + Send + 'static,
    Fut: Future<Output = Result<(), HandlerError>> + Send + 'static,
{
    tokio::spawn(run_consumer(jetstream, spec, lifecycle, handler));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spec_names_its_dead_letter_subject_after_the_consumer() {
        let spec = ConsumerSpec::new("rift.", "RIFT_USER_EVENTS", "media-projection-user", vec!["rift.user.>".into()]);
        assert_eq!(spec.dlq_subject, "rift.dlq.media-projection-user");
        let opts = spec.subscribe_options();
        assert_eq!(opts.subject, "rift.user.>");
        assert!(opts.extra_subjects.is_empty());
        assert_eq!(opts.max_deliver, Some(20));
        assert_eq!(opts.dlq_subject.as_deref(), Some("rift.dlq.media-projection-user"));
    }

    #[test]
    fn extra_subjects_follow_the_first_one_and_delivery_can_be_tuned() {
        let delivery = Delivery::new(120, Duration::from_secs(30)).with_ack_wait(Duration::from_secs(300));
        let spec = ConsumerSpec::new("rift", "S", "c", vec!["a".into(), "b".into()]).delivered(delivery);
        let opts = spec.subscribe_options();
        assert_eq!((opts.subject.as_str(), opts.extra_subjects.as_slice()), ("a", &["b".to_string()][..]));
        assert_eq!(opts.max_deliver, Some(120));
        assert_eq!(opts.ack_wait, Some(Duration::from_secs(300)));
    }
}
