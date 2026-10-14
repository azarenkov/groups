use std::sync::Arc;
use std::time::Duration;

use contracts::events::user::UserEntitlementsChangedV1;
use domain::{plan::Tier, user::id::UserId};
use futures::StreamExt;
use ports::event_consumer::{EventConsumer, StartPosition, SubscribeOptions};
use ports::plan_directory::{PlanDirectory, PlanListener, PlanListenerError};

use crate::messaging::nats::consumer::NatsEventConsumer;

const BACKLOG_CHECK_EVERY: Duration = Duration::from_secs(5);
const BUSY_CHECKS_BEFORE_LAGGING: u32 = 2;
const MAX_DELIVER: u32 = 20;
const RETRY_DELAY: Duration = Duration::from_secs(5);
const RESUBSCRIBE_DELAY: Duration = Duration::from_secs(5);

pub struct PlanProjection {
    pub jetstream: async_nats::jetstream::Context,
    pub subject_prefix: String,
    pub stream: String,
    pub consumer_name: &'static str,
    pub dlq_suffix: &'static str,
    pub plans: Arc<dyn PlanDirectory>,
    pub listener: Arc<dyn PlanListener>,
}

impl PlanProjection {
    pub fn spawn(self) {
        tokio::spawn(self.run());
    }

    async fn run(self) {
        let prefix = self.subject_prefix.trim_end_matches('.').to_string();
        let consumer = NatsEventConsumer::new(self.jetstream.clone(), self.stream.clone());
        loop {
            let opts = SubscribeOptions {
                consumer_name: self.consumer_name.to_string(),
                subject: format!("{prefix}.user.*.entitlements_changed"),
                start_position: StartPosition::All,
                max_deliver: Some(MAX_DELIVER),
                dlq_subject: Some(format!("{prefix}.dlq.{}", self.dlq_suffix)),
                extra_subjects: vec![format!("{prefix}.user.entitlements_changed")],
                ack_wait: None,
            };
            match consumer.subscribe(opts).await {
                Ok(mut messages) => {
                    tracing::info!(stream = %self.stream, consumer = self.consumer_name, "Plan projection subscribed");
                    let mut busy_checks = 0;
                    loop {
                        let msg = match tokio::time::timeout(BACKLOG_CHECK_EVERY, messages.next()).await {
                            Ok(Some(msg)) => msg,
                            Ok(None) => break,
                            Err(_) => {
                                self.check_backlog(&mut busy_checks).await;
                                continue;
                            }
                        };
                        let Ok(m) = msg else {
                            tracing::warn!(consumer = self.consumer_name, "Plan projection stream error");
                            break;
                        };
                        let backlog_drained = m.pending == Some(0);
                        if !backlog_drained && m.pending.is_some() {
                            self.plans.set_caught_up(false);
                        }
                        match self.apply(&m.payload).await {
                            Ok(()) => {
                                let _ = m.ack().await;
                                if backlog_drained {
                                    self.check_backlog(&mut busy_checks).await;
                                }
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, consumer = self.consumer_name, "Plan projection failed; scheduling retry");
                                self.plans.set_caught_up(false);
                                let _ = m.retry_after(RETRY_DELAY).await;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, consumer = self.consumer_name, "Plan projection subscribe failed; retrying");
                }
            }
            self.plans.set_caught_up(false);
            tokio::time::sleep(RESUBSCRIBE_DELAY).await;
        }
    }

    async fn apply(&self, payload: &[u8]) -> Result<(), PlanListenerError> {
        let Ok(evt) = serde_json::from_slice::<UserEntitlementsChangedV1>(payload) else {
            tracing::warn!(consumer = self.consumer_name, "Malformed entitlements event; dropping");
            return Ok(());
        };
        let user_id = UserId::new(evt.user_id);
        self.plans
            .apply_tier(&user_id, Tier::parse(&evt.tier), evt.valid_until, evt.occurred_at)
            .await?;
        self.listener.plan_applied(&user_id, chrono::Utc::now()).await
    }

    async fn check_backlog(&self, busy_checks: &mut u32) {
        match self.backlog().await {
            Ok(0) => {
                *busy_checks = 0;
                self.mark_caught_up().await;
            }
            Ok(_) => {
                *busy_checks += 1;
                if *busy_checks >= BUSY_CHECKS_BEFORE_LAGGING {
                    self.plans.set_caught_up(false);
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, consumer = self.consumer_name, "Failed to read the plan consumer backlog");
            }
        }
    }

    async fn backlog(&self) -> Result<u64, async_nats::Error> {
        let stream = self.jetstream.get_stream(&self.stream).await?;
        let info = stream.consumer_info(self.consumer_name).await?;
        Ok(info.num_pending + info.num_ack_pending as u64)
    }

    async fn mark_caught_up(&self) {
        if self.plans.is_caught_up() {
            return;
        }
        if let Err(e) = self.listener.caught_up(chrono::Utc::now()).await {
            tracing::warn!(error = %e, consumer = self.consumer_name, "Plan listener failed after catching up; will retry");
            return;
        }
        self.plans.set_caught_up(true);
        tracing::info!(consumer = self.consumer_name, "Plan projection caught up with the plan history");
    }
}
