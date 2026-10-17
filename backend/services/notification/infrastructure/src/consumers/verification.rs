use std::sync::Arc;

use api_common::health::Heartbeat;
use api_common::middleware::request_id::REQUEST_ID;
use contracts::events::user::VerificationIssuedV1;
use domain::user::{email::Email, locale::Locale};
use infrastructure_common::messaging::nats::runner::{ConsumerSpec, Delivery, Event, Lifecycle, run_consumer, subject_prefix as normalize_prefix};
use notification_core::usecases::send_verification_email::{SendVerificationEmail, VerificationIssued};
use ports::{event_consumer::StartPosition, mailer::Mailer};

use crate::inbox::{CLAIM_RENEW_EVERY, Claim, HandledEvents};

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
    let Some(event_id) = event_id else {
        return handler(payload).await;
    };
    match handled.claim(consumer, event_id).await {
        Claim::Done => {
            tracing::info!(%event_id, "Event already handled; acking the repeat");
            true
        }
        Claim::Busy => {
            tracing::info!(%event_id, "Event is being handled elsewhere; leaving it for redelivery");
            false
        }
        Claim::Unavailable => handler(payload).await,
        Claim::Acquired(token) => {
            let renewing = async {
                loop {
                    tokio::time::sleep(CLAIM_RENEW_EVERY).await;
                    handled.renew(consumer, event_id, &token).await;
                }
            };
            let succeeded = tokio::select! {
                succeeded = handler(payload) => succeeded,
                _ = renewing => false,
            };
            if succeeded {
                handled.complete(consumer, event_id, &token).await;
            } else {
                handled.release(consumer, event_id, &token).await;
            }
            succeeded
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn consume<H, Fut>(
    jetstream: async_nats::jetstream::Context,
    spec: VerificationConsumerSpec,
    event: &str,
    start_position: StartPosition,
    handled: Arc<dyn HandledEvents>,
    shutdown: tokio::sync::watch::Receiver<bool>,
    heartbeat: Heartbeat,
    handler: H,
) -> Result<(), Box<dyn std::error::Error>>
where
    H: Fn(Vec<u8>) -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let prefix = normalize_prefix(&spec.subject_prefix);
    let subjects = vec![format!("{prefix}.user.{event}"), format!("{prefix}.user.*.{event}")];
    let runner_spec = ConsumerSpec::new(&prefix, spec.stream.clone(), spec.consumer_name.clone(), subjects)
        .starting(start_position)
        .delivered(Delivery { max_deliver: spec.max_deliver, ..Delivery::DEFAULT });
    let lifecycle = Lifecycle::default().with_shutdown(shutdown, heartbeat);
    let consumer_name = spec.consumer_name.clone();
    run_consumer(jetstream, runner_spec, lifecycle, |message: Event| {
        let handled = handled.clone();
        let consumer_name = consumer_name.clone();
        let handler = &handler;
        async move {
            let once = handle_once(handled.as_ref(), &consumer_name, message.event_id.as_deref(), message.payload, handler);
            let acked = match message.request_id {
                Some(id) => REQUEST_ID.scope(id, once).await,
                None => once.await,
            };
            if acked { Ok(()) } else { Err("left for redelivery".into()) }
        }
    })
    .await;
    Ok(())
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
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use async_trait::async_trait;

    use super::*;

    #[derive(Default)]
    struct Memory(Mutex<HashMap<String, String>>);

    #[async_trait]
    impl HandledEvents for Memory {
        async fn claim(&self, consumer: &str, event_id: &str) -> Claim {
            let mut keys = self.0.lock().unwrap();
            let key = format!("{consumer}:{event_id}");
            match keys.get(&key).map(String::as_str) {
                Some("done") => Claim::Done,
                Some(_) => Claim::Busy,
                None => {
                    let token = format!("pending:{key}");
                    keys.insert(key, token.clone());
                    Claim::Acquired(token)
                }
            }
        }

        async fn renew(&self, _: &str, _: &str, _: &str) {}

        async fn complete(&self, consumer: &str, event_id: &str, token: &str) {
            let mut keys = self.0.lock().unwrap();
            let key = format!("{consumer}:{event_id}");
            if keys.get(&key).map(String::as_str) == Some(token) {
                keys.insert(key, "done".into());
            }
        }

        async fn release(&self, consumer: &str, event_id: &str, token: &str) {
            let mut keys = self.0.lock().unwrap();
            let key = format!("{consumer}:{event_id}");
            if keys.get(&key).map(String::as_str) == Some(token) {
                keys.remove(&key);
            }
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
    async fn leaves_an_event_claimed_elsewhere_for_redelivery() {
        let handled = Memory::default();
        assert!(matches!(handled.claim("mail", "e3").await, Claim::Acquired(_)));
        let sent = AtomicUsize::new(0);
        let handler = |_: Vec<u8>| {
            sent.fetch_add(1, Ordering::SeqCst);
            async { true }
        };
        assert!(!handle_once(&handled, "mail", Some("e3"), vec![], &handler).await);
        assert_eq!(sent.load(Ordering::SeqCst), 0);
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
