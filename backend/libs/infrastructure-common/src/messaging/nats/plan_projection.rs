use std::sync::{Arc, Mutex};
use std::time::Duration;

use contracts::events::user::UserEntitlementsChangedV1;
use domain::{plan::Tier, user::id::UserId};
use futures::FutureExt;
use ports::plan_directory::{PlanDirectory, PlanListener, PlanListenerError};

use crate::messaging::nats::runner::{ConsumerSpec, Event, Lifecycle, spawn_consumer, subject_prefix};

const BACKLOG_CHECK_EVERY: Duration = Duration::from_secs(5);
const BUSY_CHECKS_BEFORE_LAGGING: u32 = 2;

pub struct PlanProjection {
    pub jetstream: async_nats::jetstream::Context,
    pub subject_prefix: String,
    pub stream: String,
    pub consumer_name: &'static str,
    pub plans: Arc<dyn PlanDirectory>,
    pub listener: Arc<dyn PlanListener>,
}

struct Backlog {
    projection: PlanProjection,
    busy_checks: Mutex<u32>,
}

impl PlanProjection {
    pub fn spawn(self) {
        let prefix = subject_prefix(&self.subject_prefix);
        let spec = ConsumerSpec::new(
            &prefix,
            self.stream.clone(),
            self.consumer_name,
            vec![format!("{prefix}.user.*.entitlements_changed"), format!("{prefix}.user.entitlements_changed")],
        );
        let jetstream = self.jetstream.clone();
        let backlog = Arc::new(Backlog { projection: self, busy_checks: Mutex::new(0) });
        let idle = backlog.clone();
        let disconnected = backlog.clone();
        let lifecycle = Lifecycle::default()
            .when_idle(BACKLOG_CHECK_EVERY, Arc::new(move || {
                let idle = idle.clone();
                async move { idle.check_backlog().await }.boxed()
            }))
            .on_disconnected(move || disconnected.projection.plans.set_caught_up(false));
        spawn_consumer(jetstream, spec, lifecycle, move |event: Event| {
            let backlog = backlog.clone();
            async move {
                let drained = event.pending == Some(0);
                if !drained && event.pending.is_some() {
                    backlog.projection.plans.set_caught_up(false);
                }
                if let Err(e) = backlog.projection.apply(&event.payload).await {
                    backlog.projection.plans.set_caught_up(false);
                    return Err(e);
                }
                if drained {
                    backlog.check_backlog().await;
                }
                Ok(())
            }
        });
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

impl Backlog {
    async fn check_backlog(&self) {
        match self.projection.backlog().await {
            Ok(0) => {
                *self.busy_checks.lock().unwrap() = 0;
                self.projection.mark_caught_up().await;
            }
            Ok(_) => {
                let mut busy = self.busy_checks.lock().unwrap();
                *busy += 1;
                if *busy >= BUSY_CHECKS_BEFORE_LAGGING {
                    self.projection.plans.set_caught_up(false);
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, consumer = self.projection.consumer_name, "Failed to read the plan consumer backlog");
            }
        }
    }
}
