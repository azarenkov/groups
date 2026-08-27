use std::{error::Error, sync::Arc};

use ports::{
    event_consumer::{ConsumedMessage, EventConsumer, StartPosition, SubscribeOptions},
    mailer::Mailer,
};
use identity_domain::verification::events::VerificationEvent;
use futures::StreamExt;
use infrastructure_common::{
    config::{mailer::MailerConfig, nats::NatsConfig},
    mailer::smtp_mailer::SmtpMailer,
    messaging::nats::{
        connection::{create_jetstream, is_nats_connected},
        consumer::NatsEventConsumer,
    },
};
use api_common::health::{self, Readiness};
use api_common::middleware::request_id::REQUEST_ID;
use tracing::Instrument;
use tokio::signal;

use crate::config::NotificationConfig;

mod config;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    infrastructure_common::observability::init("notification-service");

    let notification_config = NotificationConfig::from_env()?;
    let nats_config = NatsConfig::from_env()?;
    let mailer_config = MailerConfig::from_env()?;

    let jetstream = create_jetstream(&nats_config).await?;

    let readiness = Readiness::builder()
        .add_probe({
            let jetstream = jetstream.clone();
            move || {
                let jetstream = jetstream.clone();
                async move { is_nats_connected(&jetstream) }
            }
        })
        .build();
    let health_server = health::bind(notification_config.health_port, readiness.clone())?;
    let health_handle = actix_web::rt::spawn(async move {
        if let Err(e) = health_server.await {
            tracing::error!(error = %e, "Health server stopped");
        }
    });
    let mailer: Arc<SmtpMailer> = Arc::new(SmtpMailer::from_config(mailer_config)?);

    let event_consumer = NatsEventConsumer::new(
        jetstream,
        notification_config.notification_user_stream.clone(),
    );

    let filter_subject = format!("{}.user.verification.issued", nats_config.subject_prefix);
    let partitioned_filter_subject = format!(
        "{}.user.*.verification.issued",
        nats_config.subject_prefix
    );
    let dlq_subject = format!(
        "{}.dlq.user.verification.issued",
        nats_config.subject_prefix
    );

    let mut messages = event_consumer
        .subscribe(SubscribeOptions {
            consumer_name: notification_config
                .notification_verification_consumer
                .clone(),
            subject: filter_subject.clone(),
            start_position: StartPosition::New,
            max_deliver: Some(notification_config.notification_max_deliver),
            dlq_subject: Some(dlq_subject.clone()),
            extra_subjects: vec![partitioned_filter_subject],
        })
        .await?;

    tracing::info!(
        stream = %notification_config.notification_user_stream,
        consumer = %notification_config.notification_verification_consumer,
        subject = %filter_subject,
        dlq = %dlq_subject,
        max_deliver = notification_config.notification_max_deliver,
        "Notification service started",
    );

    readiness.mark_ready();

    let shutdown = signal::ctrl_c();
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            _ = &mut shutdown => {
                tracing::info!("Shutdown signal received, exiting");
                break;
            }
            msg = messages.next() => {
                match msg {
                    Some(Ok(msg)) => {
                        let request_id = msg.request_id.clone();
                        let span = tracing::info_span!(
                            "nats_message",
                            request_id = ?request_id,
                        );
                        let fut = handle_verification_message(mailer.as_ref(), msg).instrument(span);
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
                        break;
                    }
                }
            }
        }
    }

    health_handle.abort();

    Ok(())
}

async fn handle_verification_message<M: Mailer>(mailer: &M, msg: ConsumedMessage) {
    let event: VerificationEvent = match serde_json::from_slice(&msg.payload) {
        Ok(e) => e,
        Err(e) => {
            tracing::error!(error = %e, "Failed to deserialize verification event; not acking, JetStream will redeliver");
            return;
        }
    };

    let VerificationEvent::Issued(issued) = event;
    let email_str = issued.email.as_str().to_string();

    match mailer
        .send_verification_code(&issued.email, &issued.code, issued.locale)
        .await
    {
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
