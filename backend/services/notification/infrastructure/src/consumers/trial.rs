use std::sync::Arc;

use api_common::health::Heartbeat;
use chrono::Utc;
use contracts::events::user::UserTrialEndingV1;
use domain::user::{email::Email, locale::Locale};
use notification_core::usecases::send_trial_ending_email::{SendTrialEndingEmail, TrialEnding};
use ports::{event_consumer::ConsumedMessage, mailer::Mailer};

use super::verification::{VerificationConsumerSpec, consume};

pub async fn run<M: Mailer>(
    jetstream: async_nats::jetstream::Context,
    spec: VerificationConsumerSpec,
    usecase: Arc<SendTrialEndingEmail<M>>,
    shutdown: tokio::sync::watch::Receiver<bool>,
    heartbeat: Heartbeat,
) -> Result<(), Box<dyn std::error::Error>> {
    consume(jetstream, spec, "trial_ending", shutdown, heartbeat, |msg| {
        let usecase = usecase.clone();
        async move { handle(usecase.as_ref(), msg).await }
    })
    .await
}

async fn handle<M: Mailer>(usecase: &SendTrialEndingEmail<M>, msg: ConsumedMessage) {
    let event: UserTrialEndingV1 = match serde_json::from_slice(&msg.payload) {
        Ok(e) => e,
        Err(e) => {
            tracing::error!(error = %e, "Malformed trial ending event; acking to drop it");
            if let Err(e) = msg.ack().await {
                tracing::error!(error = %e, "Failed to ack message");
            }
            return;
        }
    };
    let user_id = event.user_id;
    let ending = TrialEnding {
        email: Email::new(event.email),
        ends_at: event.ends_at,
        locale: event
            .locale
            .as_deref()
            .map(Locale::from_str_lenient)
            .unwrap_or_default(),
    };
    match usecase.execute(ending, Utc::now()).await {
        Ok(()) => {
            tracing::info!(%user_id, "Trial ending email sent");
            if let Err(e) = msg.ack().await {
                tracing::error!(error = %e, "Failed to ack message");
            }
        }
        Err(e) => {
            tracing::error!(error = %e, %user_id, "Failed to send trial ending email; redelivery will retry");
        }
    }
}
