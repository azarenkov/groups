use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use contracts::events::user::UserEntitlementsChangedV1;
use domain::{plan::Tier, user::id::UserId};
use futures::StreamExt;
use infrastructure_common::messaging::nats::consumer::NatsEventConsumer;
use voice::ports::event_consumer::{EventConsumer, StartPosition, SubscribeOptions};
use voice::ports::plan_directory::PlanDirectory;
use voice::ports::voice_service::VoiceService;
use voice::usecases::stream::apply_plan_change::{ApplyPlanChange, PlanChange};
use voice_domain::call::repository::CallRepository;

const CAUGHT_UP_AFTER_IDLE: Duration = Duration::from_secs(5);

pub fn spawn<P, C, V>(
    jetstream: async_nats::jetstream::Context,
    subject_prefix: String,
    stream: String,
    usecase: Arc<ApplyPlanChange<P, C, V>>,
    caught_up: Arc<AtomicBool>,
) where
    P: PlanDirectory + 'static,
    C: CallRepository + Send + Sync + 'static,
    V: VoiceService + Send + Sync + 'static,
{
    actix_web::rt::spawn(async move {
        let prefix = subject_prefix.trim_end_matches('.').to_string();
        let consumer = NatsEventConsumer::new(jetstream, stream.clone());
        loop {
            let opts = SubscribeOptions {
                consumer_name: "voice-projection-tiers".to_string(),
                subject: format!("{prefix}.user.*.entitlements_changed"),
                start_position: StartPosition::All,
                max_deliver: Some(20),
                dlq_subject: Some(format!("{prefix}.dlq.voice.tiers")),
                extra_subjects: vec![format!("{prefix}.user.entitlements_changed")],
                ack_wait: None,
            };
            match consumer.subscribe(opts).await {
                Ok(mut messages) => {
                    tracing::info!(%stream, "Voice tier projection subscribed");
                    loop {
                        let msg = match tokio::time::timeout(CAUGHT_UP_AFTER_IDLE, messages.next()).await {
                            Ok(Some(msg)) => msg,
                            Ok(None) => break,
                            Err(_) => {
                                if !caught_up.swap(true, Ordering::SeqCst) {
                                    tracing::info!("Voice tier projection caught up with the plan history");
                                }
                                continue;
                            }
                        };
                        let Ok(m) = msg else {
                            tracing::warn!("Voice tier projection stream error");
                            break;
                        };
                        let Ok(evt) = serde_json::from_slice::<UserEntitlementsChangedV1>(&m.payload) else {
                            tracing::warn!("Malformed entitlements event; dropping");
                            let _ = m.ack().await;
                            continue;
                        };
                        let change = PlanChange {
                            user_id: UserId::new(evt.user_id),
                            tier: Tier::parse(&evt.tier),
                            valid_until: evt.valid_until,
                            occurred_at: evt.occurred_at,
                        };
                        match usecase.execute(change, chrono::Utc::now()).await {
                            Ok(()) => {
                                let _ = m.ack().await;
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, "Voice tier projection failed; scheduling retry");
                                let _ = m.retry_after(std::time::Duration::from_secs(5)).await;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Voice tier projection subscribe failed; retrying");
                }
            }
            caught_up.store(false, Ordering::SeqCst);
            actix_web::rt::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });
}
