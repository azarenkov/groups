use std::{error::Error, sync::Arc};

use api_common::health::{self, Readiness};
use infrastructure_common::{
    config::{mailer::MailerConfig, nats::NatsConfig},
    mailer::smtp_mailer::SmtpMailer,
    messaging::nats::connection::{create_jetstream, is_nats_connected},
};
use notification_core::usecases::send_verification_email::SendVerificationEmail;
use notification_infrastructure::consumers::verification::{self, VerificationConsumerSpec};
use tokio::{signal, sync::watch};

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

    let mailer = Arc::new(SmtpMailer::from_config(mailer_config)?);
    let send_verification_email = Arc::new(SendVerificationEmail::new(mailer));
    let spec = VerificationConsumerSpec {
        stream: notification_config.notification_user_stream.clone(),
        consumer_name: notification_config.notification_verification_consumer.clone(),
        subject_prefix: nats_config.subject_prefix.clone(),
        max_deliver: notification_config.notification_max_deliver,
    };

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let consumer = actix_web::rt::spawn(verification::run(
        jetstream,
        spec,
        send_verification_email,
        shutdown_rx,
    ));
    tokio::pin!(consumer);

    tracing::info!("Notification service started");
    readiness.mark_ready();

    let outcome = tokio::select! {
        signal = signal::ctrl_c() => {
            signal?;
            tracing::info!("Shutdown signal received, exiting");
            let _ = shutdown_tx.send(true);
            match consumer.await {
                Ok(result) => result,
                Err(e) => Err(Box::<dyn Error>::from(e)),
            }
        }
        finished = &mut consumer => match finished {
            Ok(Ok(())) => Err(Box::<dyn Error>::from("verification consumer ended unexpectedly")),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(Box::<dyn Error>::from(e)),
        },
    };

    health_handle.abort();
    outcome
}
