use std::{error::Error, sync::Arc};

use application::ports::{
    event_consumer::{ConsumedMessage, EventConsumer, StartPosition, SubscribeOptions},
    mailer::Mailer,
};
use domain::verification::events::VerificationEvent;
use futures::StreamExt;
use infrastructure::{
    config::{mailer::MailerConfig, nats::NatsConfig},
    mailer::smtp_mailer::SmtpMailer,
    messaging::nats::{connection::create_jetstream, consumer::NatsEventConsumer},
};
use tokio::signal;

use crate::config::NotificationConfig;

mod config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let notification_config = NotificationConfig::from_env()?;
    let nats_config = NatsConfig::from_env()?;
    let mailer_config = MailerConfig::from_env()?;

    let jetstream = create_jetstream(&nats_config).await?;
    let mailer: Arc<SmtpMailer> = Arc::new(SmtpMailer::from_config(mailer_config)?);

    let event_consumer = NatsEventConsumer::new(
        jetstream,
        notification_config.notification_user_stream.clone(),
    );

    let filter_subject = format!("{}.user.verification.issued", nats_config.subject_prefix);
    let dlq_subject = format!("{}.dlq.user.verification.issued", nats_config.subject_prefix);

    let mut messages = event_consumer
        .subscribe(SubscribeOptions {
            consumer_name: notification_config.notification_verification_consumer.clone(),
            subject: filter_subject.clone(),
            start_position: StartPosition::New,
            max_deliver: Some(notification_config.notification_max_deliver),
            dlq_subject: Some(dlq_subject.clone()),
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
                        handle_verification_message(mailer.as_ref(), msg).await;
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
        .send_verification_code(&issued.email, &issued.code)
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
