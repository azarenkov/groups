use std::sync::Arc;

use api_common::health::Heartbeat;
use api_common::middleware::request_id::REQUEST_ID;
use contracts::events::user::VerificationIssuedV1;
use domain::user::{email::Email, locale::Locale};
use futures::StreamExt;
use infrastructure_common::messaging::nats::consumer::NatsEventConsumer;
use notification_core::usecases::send_verification_email::{SendVerificationEmail, VerificationIssued};
use ports::{event_consumer::EventConsumer, event_consumer::StartPosition, event_consumer::SubscribeOptions, mailer::Mailer};

use crate::inbox::HandledEvents;
use tracing::Instrument;

#[derive(Clone)]
pub struct VerificationConsumerSpec {
    pub stream: String,
    pub consumer_name: String,
    pub subject_prefix: String,
    pub max_deliver: u32,
}

pub async fn run<M: Mailer>(
    jetstream: async_nats::jetstream::Context,
    spec: VerificationConsumerSpec,
    usecase: Arc<SendVerificationEmail<M>>,
    handled: Arc<dyn HandledEvents>,
    shutdown: tokio::sync::watch::Receiver<bool>,
    heartbeat: Heartbeat,
) -> Result<(), Box<dyn std::error::Error>> {
    consume(jetstream, spec, "verification.issued", StartPosition::New, handled, shutdown, heartbeat, |payload| {
        let usecase = usecase.clone();
        async move { handle(usecase.as_ref(), &payload).await }
    })
    .await
}

pub async fn handle_once<H, Fut>(
    handled: &dyn HandledEvents,
    consumer: &str,
    event_id: Option<&str>,
    payload: Vec<u8>,
    handler: &H,
) -> bool
where
    H: Fn(Vec<u8>) -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    if let Some(event_id) = event_id
        && handled.already_handled(consumer, event_id).await
    {
        tracing::info!(%event_id, "Event already handled; acking the repeat");
        return true;
    }
    if !handler(payload).await {
        return false;
    }
    if let Some(event_id) = event_id {
        handled.remember(consumer, event_id).await;
    }
    true
}

#[allow(clippy::too_many_arguments)]
pub async fn consume<H, Fut>(
    jetstream: async_nats::jetstream::Context,
    spec: VerificationConsumerSpec,
    event: &str,
    start_position: StartPosition,
    handled: Arc<dyn HandledEvents>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
    heartbeat: Heartbeat,
    handler: H,
) -> Result<(), Box<dyn std::error::Error>>
where
    H: Fn(Vec<u8>) -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let filter_subject = format!("{}.user.{event}", spec.subject_prefix);
    let partitioned_filter_subject = format!("{}.user.*.{event}", spec.subject_prefix);
    let dlq_subject = format!("{}.dlq.user.{event}", spec.subject_prefix);

    let consumer = NatsEventConsumer::new(jetstream, spec.stream.clone());
    let mut messages = consumer
        .subscribe(SubscribeOptions {
            consumer_name: spec.consumer_name.clone(),
            subject: filter_subject.clone(),
            start_position,
            max_deliver: Some(spec.max_deliver),
            dlq_subject: Some(dlq_subject.clone()),
            extra_subjects: vec![partitioned_filter_subject],
            ack_wait: None,
        })
        .await?;

    tracing::info!(
        stream = %spec.stream,
        consumer = %spec.consumer_name,
        subject = %filter_subject,
        dlq = %dlq_subject,
        max_deliver = spec.max_deliver,
        "Mail consumer subscribed",
    );

    let mut pulse = tokio::time::interval(heartbeat.pulse_interval());
    pulse.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        heartbeat.beat();
        tokio::select! {
            _ = shutdown.changed() => return Ok(()),
            _ = pulse.tick() => {}
            msg = messages.next() => {
                match msg {
                    Some(Ok(msg)) => {
                        let request_id = msg.request_id.clone();
                        let span = tracing::info_span!("nats_message", request_id = ?request_id);
                        let event_id = msg.event_id.clone();
                        let payload = msg.payload.clone();
                        let fut = handle_once(handled.as_ref(), &spec.consumer_name, event_id.as_deref(), payload, &handler)
                            .instrument(span);
                        let ack = match request_id {
                            Some(id) => REQUEST_ID.scope(id, fut).await,
                            None => fut.await,
                        };
                        if ack && let Err(e) = msg.ack().await {
                            tracing::error!(error = %e, "Failed to ack message");
                        }
                    }
                    Some(Err(e)) => {
                        tracing::error!(error = %e, "Consumer stream error");
                    }
                    None => {
                        tracing::warn!("Message stream ended unexpectedly");
                        return Ok(());
                    }
                }
            }
        }
    }
}

async fn handle<M: Mailer>(usecase: &SendVerificationEmail<M>, payload: &[u8]) -> bool {
    let issued: VerificationIssuedV1 = match serde_json::from_slice(payload) {
        Ok(e) => e,
        Err(e) => {
            tracing::error!(error = %e, "Failed to deserialize verification event; not acking, JetStream will redeliver");
            return false;
        }
    };
    let email_str = issued.email.clone();
    let issued = VerificationIssued {
        email: Email::new(issued.email),
        code: issued.code,
        purpose: issued.purpose,
        locale: issued
            .locale
            .as_deref()
            .map(Locale::from_str_lenient)
            .unwrap_or_default(),
    };
    match usecase.execute(issued).await {
        Ok(()) => {
            tracing::info!(email = %email_str, "Verification email sent");
            true
        }
        Err(e) => {
            tracing::error!(error = %e, email = %email_str, "Failed to send verification email; redelivery will retry");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use async_trait::async_trait;

    use super::*;

    #[derive(Default)]
    struct Memory(Mutex<HashSet<String>>);

    #[async_trait]
    impl HandledEvents for Memory {
        async fn already_handled(&self, consumer: &str, event_id: &str) -> bool {
            self.0.lock().unwrap().contains(&format!("{consumer}:{event_id}"))
        }

        async fn remember(&self, consumer: &str, event_id: &str) {
            self.0.lock().unwrap().insert(format!("{consumer}:{event_id}"));
        }
    }

    #[tokio::test]
    async fn sends_once_when_the_same_event_is_delivered_twice() {
        let handled = Memory::default();
        let sent = AtomicUsize::new(0);
        let handler = |_: Vec<u8>| {
            sent.fetch_add(1, Ordering::SeqCst);
            async { true }
        };
        assert!(handle_once(&handled, "mail", Some("e1"), vec![], &handler).await);
        assert!(handle_once(&handled, "mail", Some("e1"), vec![], &handler).await);
        assert!(handle_once(&handled, "other", Some("e1"), vec![], &handler).await);
        assert_eq!(sent.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn retries_an_event_whose_handling_failed() {
        let handled = Memory::default();
        let attempts = AtomicUsize::new(0);
        let handler = |_: Vec<u8>| {
            let attempt = attempts.fetch_add(1, Ordering::SeqCst);
            async move { attempt > 0 }
        };
        assert!(!handle_once(&handled, "mail", Some("e2"), vec![], &handler).await);
        assert!(handle_once(&handled, "mail", Some("e2"), vec![], &handler).await);
        assert!(handle_once(&handled, "mail", Some("e2"), vec![], &handler).await);
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn handles_events_without_an_id_every_time() {
        let handled = Memory::default();
        let sent = AtomicUsize::new(0);
        let handler = |_: Vec<u8>| {
            sent.fetch_add(1, Ordering::SeqCst);
            async { true }
        };
        handle_once(&handled, "mail", None, vec![], &handler).await;
        handle_once(&handled, "mail", None, vec![], &handler).await;
        assert_eq!(sent.load(Ordering::SeqCst), 2);
    }
}
