use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, Instant};

use contracts::events::user::UserEntitlementsChangedV1;
use domain::{plan::Tier, user::id::UserId};
use futures::StreamExt;
use ports::event_consumer::{EventConsumer, StartPosition, SubscribeOptions};
use ports::plan_directory::{PlanDirectory, PlanListener, PlanListenerError};

use crate::messaging::nats::consumer::NatsEventConsumer;

const CAUGHT_UP_AFTER_IDLE: Duration = Duration::from_secs(5);
const MAX_DELIVER: u32 = 20;
const RETRY_DELAY: Duration = Duration::from_secs(5);
const RESUBSCRIBE_DELAY: Duration = Duration::from_secs(5);
const RETRY_WINDOW: Duration = Duration::from_secs(5 * (MAX_DELIVER as u64 + 1));

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
                    let mut unresolved: HashMap<u64, Instant> = HashMap::new();
                    loop {
                        let msg = match tokio::time::timeout(CAUGHT_UP_AFTER_IDLE, messages.next()).await {
                            Ok(Some(msg)) => msg,
                            Ok(None) => break,
                            Err(_) => {
                                self.mark_caught_up(&mut unresolved).await;
                                continue;
                            }
                        };
                        let Ok(m) = msg else {
                            tracing::warn!(consumer = self.consumer_name, "Plan projection stream error");
                            break;
                        };
                        let backlog_drained = m.pending == Some(0);
                        if m.pending.is_some_and(|pending| pending > 0) {
                            self.plans.set_caught_up(false);
                        }
                        let delivery = delivery_key(m.event_id.as_deref(), &m.payload);
                        match self.apply(&m.payload).await {
                            Ok(()) => {
                                let _ = m.ack().await;
                                unresolved.remove(&delivery);
                                if backlog_drained {
                                    self.mark_caught_up(&mut unresolved).await;
                                }
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, consumer = self.consumer_name, "Plan projection failed; scheduling retry");
                                unresolved.insert(delivery, Instant::now());
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

    async fn mark_caught_up(&self, unresolved: &mut HashMap<u64, Instant>) {
        unresolved.retain(|_, failed_at| failed_at.elapsed() < RETRY_WINDOW);
        if self.plans.is_caught_up() || !unresolved.is_empty() {
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

fn delivery_key(event_id: Option<&str>, payload: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    match event_id {
        Some(id) => id.hash(&mut hasher),
        None => payload.hash(&mut hasher),
    }
    hasher.finish()
}
