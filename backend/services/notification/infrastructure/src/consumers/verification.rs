use std::sync::Arc;

use api_common::middleware::request_id::REQUEST_ID;
use contracts::events::user::VerificationIssuedV1;
use domain::user::{email::Email, locale::Locale};
use futures::StreamExt;
use infrastructure_common::messaging::nats::consumer::NatsEventConsumer;
use notification_core::usecases::send_verification_email::{SendVerificationEmail, VerificationIssued};
use ports::{event_consumer::ConsumedMessage, event_consumer::EventConsumer, event_consumer::StartPosition, event_consumer::SubscribeOptions, mailer::Mailer};
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
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let filter_subject = format!("{}.user.verification.issued", spec.subject_prefix);
    let partitioned_filter_subject = format!("{}.user.*.verification.issued", spec.subject_prefix);
    let dlq_subject = format!("{}.dlq.user.verification.issued", spec.subject_prefix);

    let consumer = NatsEventConsumer::new(jetstream, spec.stream.clone());
    let mut messages = consumer
        .subscribe(SubscribeOptions {
            consumer_name: spec.consumer_name.clone(),
            subject: filter_subject.clone(),
            start_position: StartPosition::New,
            max_deliver: Some(spec.max_deliver),
            dlq_subject: Some(dlq_subject.clone()),
            extra_subjects: vec![partitioned_filter_subject],
        })
        .await?;

    tracing::info!(
        stream = %spec.stream,
        consumer = %spec.consumer_name,
        subject = %filter_subject,
        dlq = %dlq_subject,
        max_deliver = spec.max_deliver,
        "Verification consumer subscribed",
    );

    loop {
        tokio::select! {
            _ = shutdown.changed() => return Ok(()),
            msg = messages.next() => {
                match msg {
                    Some(Ok(msg)) => {
                        let request_id = msg.request_id.clone();
                        let span = tracing::info_span!("nats_message", request_id = ?request_id);
                        let fut = handle(usecase.as_ref(), msg).instrument(span);
                        match request_id {
                            Some(id) => REQUEST_ID.scope(id, fut).await,
                            None => fut.await,
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

async fn handle<M: Mailer>(usecase: &SendVerificationEmail<M>, msg: ConsumedMessage) {
    let issued: VerificationIssuedV1 = match serde_json::from_slice(&msg.payload) {
        Ok(e) => e,
        Err(e) => {
            tracing::error!(error = %e, "Failed to deserialize verification event; not acking, JetStream will redeliver");
            return;
        }
    };
    let email_str = issued.email.clone();
    let issued = VerificationIssued {
        email: Email::new(issued.email),
        code: issued.code,
        locale: issued
            .locale
            .as_deref()
            .map(Locale::from_str_lenient)
            .unwrap_or_default(),
    };
    match usecase.execute(issued).await {
        Ok(()) => {
            tracing::info!(email = %email_str, "Verification email sent");
            if let Err(e) = msg.ack().await {
                tracing::error!(error = %e, "Failed to ack message");
            }
        }
        Err(e) => {
            tracing::error!(error = %e, email = %email_str, "Failed to send verification email; redelivery will retry");
        }
    }
}
