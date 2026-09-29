use std::{error::Error, sync::Arc, time::Duration};

use api_common::health::{self, Heartbeat, Liveness, Readiness};
use api_common::shutdown::{ShutdownConfig, wait_for_signal};
use infrastructure_common::{
    config::{mailer::MailerConfig, nats::NatsConfig},
    mailer::smtp_mailer::SmtpMailer,
    messaging::nats::connection::{create_jetstream, is_nats_connected},
};
use notification_core::usecases::send_trial_ending_email::SendTrialEndingEmail;
use notification_core::usecases::send_verification_email::SendVerificationEmail;
use notification_infrastructure::consumers::trial;
use notification_infrastructure::consumers::verification::{self, VerificationConsumerSpec};
use tokio::sync::watch;

use crate::config::NotificationConfig;

mod config;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    if let Some(code) = api_common::health::run_probe_from_args() {
        std::process::exit(code);
    }
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
    let stale_after = Duration::from_secs(notification_config.notification_liveness_stale_secs);
    let verification_heartbeat = Heartbeat::new(stale_after);
    let trial_heartbeat = Heartbeat::new(stale_after);
    let health_server = health::bind(
        notification_config.health_port,
        readiness.clone(),
        Liveness::default()
            .watch(verification_heartbeat.clone())
            .watch(trial_heartbeat.clone()),
    )?;
    let health_server_handle = health_server.handle();
    let health_handle = actix_web::rt::spawn(async move {
        if let Err(e) = health_server.await {
            tracing::error!(error = %e, "Health community stopped");
        }
    });

    let mailer = Arc::new(SmtpMailer::from_config(mailer_config)?);
    let send_verification_email = Arc::new(SendVerificationEmail::new(mailer.clone()));
    let send_trial_ending_email = Arc::new(SendTrialEndingEmail::new(mailer));
    let spec = VerificationConsumerSpec {
        stream: notification_config.notification_user_stream.clone(),
        consumer_name: notification_config.notification_verification_consumer.clone(),
        subject_prefix: nats_config.subject_prefix.clone(),
        max_deliver: notification_config.notification_max_deliver,
    };

    let trial_spec = VerificationConsumerSpec {
        consumer_name: notification_config.notification_trial_consumer.clone(),
        ..spec.clone()
    };
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let mut verification_consumer = actix_web::rt::spawn(verification::run(
        jetstream.clone(),
        spec,
        send_verification_email,
        shutdown_rx.clone(),
        verification_heartbeat,
    ));
    let mut trial_consumer = actix_web::rt::spawn(trial::run(
        jetstream,
        trial_spec,
        send_trial_ending_email,
        shutdown_rx,
        trial_heartbeat,
    ));


    tracing::info!("Notification service started");
    readiness.mark_ready();

    let shutdown_config = ShutdownConfig::from_env();
    let outcome = tokio::select! {
        _ = wait_for_signal() => {
            readiness.mark_not_ready();
            let _ = shutdown_tx.send(true);
            let (verification, trial, _) = tokio::join!(
                &mut verification_consumer,
                &mut trial_consumer,
                tokio::time::sleep(shutdown_config.drain_delay),
            );
            stopped(verification).and(stopped(trial))
        }
        finished = &mut verification_consumer => ended("verification", finished),
        finished = &mut trial_consumer => ended("trial ending", finished),
    };

    readiness.mark_not_ready();
    health_server_handle.stop(false).await;
    let _ = health_handle.await;
    outcome
}

type ConsumerResult = Result<Result<(), Box<dyn Error>>, tokio::task::JoinError>;

fn stopped(finished: ConsumerResult) -> Result<(), Box<dyn Error>> {
    match finished {
        Ok(result) => result,
        Err(e) => Err(Box::<dyn Error>::from(e)),
    }
}

fn ended(name: &str, finished: ConsumerResult) -> Result<(), Box<dyn Error>> {
    match finished {
        Ok(Ok(())) => Err(Box::<dyn Error>::from(format!("{name} consumer ended unexpectedly"))),
        Ok(Err(e)) => Err(e),
        Err(e) => Err(Box::<dyn Error>::from(e)),
    }
}
