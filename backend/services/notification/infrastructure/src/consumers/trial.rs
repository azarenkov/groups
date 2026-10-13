use std::sync::Arc;

use api_common::health::Heartbeat;
use chrono::Utc;
use contracts::events::user::UserTrialEndingV1;
use domain::user::id::UserId;
use notification_core::ports::trial_state::TrialState;
use notification_core::usecases::send_trial_ending_email::{SendTrialEndingEmail, TrialEnding};
use ports::{event_consumer::StartPosition, mailer::Mailer};

use super::verification::{VerificationConsumerSpec, consume};
use crate::inbox::HandledEvents;

pub async fn run<M: Mailer, T: TrialState>(
    jetstream: async_nats::jetstream::Context,
    spec: VerificationConsumerSpec,
    usecase: Arc<SendTrialEndingEmail<M, T>>,
    handled: Arc<dyn HandledEvents>,
    shutdown: tokio::sync::watch::Receiver<bool>,
    heartbeat: Heartbeat,
) -> Result<(), Box<dyn std::error::Error>> {
    consume(jetstream, spec, "trial_ending", StartPosition::All, handled, shutdown, heartbeat, |payload| {
        let usecase = usecase.clone();
        async move { handle(usecase.as_ref(), &payload).await }
    })
    .await
}

async fn handle<M: Mailer, T: TrialState>(usecase: &SendTrialEndingEmail<M, T>, payload: &[u8]) -> bool {
    let event: UserTrialEndingV1 = match serde_json::from_slice(payload) {
        Ok(e) => e,
        Err(e) => {
            tracing::error!(error = %e, "Malformed trial ending event; acking to drop it");
            return true;
        }
    };
    let user_id = event.user_id;
    let ending = TrialEnding {
        user_id: UserId::new(user_id),
        ends_at: event.ends_at,
    };
    match usecase.execute(ending, Utc::now()).await {
        Ok(sent) => {
            if sent {
                tracing::info!(%user_id, "Trial ending email sent");
            } else {
                tracing::info!(%user_id, "Trial ending reminder is stale; skipped");
            }
            true
        }
        Err(e) => {
            tracing::error!(error = %e, %user_id, "Failed to send trial ending email; redelivery will retry");
            false
        }
    }
}
