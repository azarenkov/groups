use std::{error::Error, sync::Arc};

use application::ports::mailer::Mailer;
use async_nats::jetstream::{
    self,
    consumer::{AckPolicy, Consumer, pull},
};
use domain::user::email::Email;
use futures::StreamExt;
use infrastructure::{
    config::{mailer::MailerConfig, nats::NatsConfig},
    mailer::smtp_mailer::SmtpMailer,
    messaging::nats::connection::create_jetstream,
};
use serde::Deserialize;
use tokio::signal;

use crate::config::NotificationConfig;

mod config;

#[derive(Debug, Deserialize)]
enum VerificationEvent {
    Issued(VerificationIssued),
}

#[derive(Debug, Deserialize)]
struct VerificationIssued {
    email: String,
    code: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let notification_config = NotificationConfig::from_env()?;
    let nats_config = NatsConfig::from_env()?;
    let mailer_config = MailerConfig::from_env()?;

    let jetstream = create_jetstream(&nats_config).await?;
    let mailer: Arc<SmtpMailer> = Arc::new(SmtpMailer::from_config(mailer_config)?);

    let stream = jetstream
        .get_stream(&notification_config.notification_user_stream)
        .await?;
    let filter_subject = format!("{}.user.verification.issued", nats_config.subject_prefix);

    let consumer: Consumer<pull::Config> = stream
        .get_or_create_consumer(
            &notification_config.notification_verification_consumer,
            pull::Config {
                durable_name: Some(
                    notification_config.notification_verification_consumer.clone(),
                ),
                filter_subject: filter_subject.clone(),
                ack_policy: AckPolicy::Explicit,
                ..Default::default()
            },
        )
        .await?;

    tracing::info!(
        stream = %notification_config.notification_user_stream,
        consumer = %notification_config.notification_verification_consumer,
        subject = %filter_subject,
        "Notification service started",
    );

    let shutdown = signal::ctrl_c();
    tokio::pin!(shutdown);

    let mut messages = consumer.messages().await?;

    loop {
        tokio::select! {
            _ = &mut shutdown => {
                tracing::info!("Shutdown signal received, exiting");
                break;
            }
            msg = messages.next() => {
                match msg {
                    Some(Ok(msg)) => {
                        handle_verification_message(mailer.as_ref(), &msg).await;
                    }
                    Some(Err(e)) => {
                        tracing::error!(error = %e, "NATS message error");
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

async fn handle_verification_message<M: Mailer>(mailer: &M, msg: &jetstream::Message) {
    let event: VerificationEvent = match serde_json::from_slice(&msg.payload) {
        Ok(e) => e,
        Err(e) => {
            tracing::error!(error = %e, "Failed to deserialize verification event; acking to skip");
            let _ = msg.ack().await;
            return;
        }
    };

    let VerificationEvent::Issued(issued) = event;
    let email_str = issued.email.clone();

    match mailer
        .send_verification_code(&Email::new(issued.email), &issued.code)
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
