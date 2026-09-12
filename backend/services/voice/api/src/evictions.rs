use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use domain::channel::id::ChannelId;
use voice::ports::voice_service::VoiceService;
use voice_domain::call::repository::CallRepository;

const POLL_INTERVAL: Duration = Duration::from_secs(10);
const CLAIM_BATCH: i64 = 50;

pub fn spawn<C, V>(call_repository: Arc<C>, voice_service: Arc<V>)
where
    C: CallRepository + Send + Sync + 'static,
    V: VoiceService + Send + Sync + 'static,
{
    actix_web::rt::spawn(async move {
        loop {
            actix_web::rt::time::sleep(POLL_INTERVAL).await;
            let now = Utc::now();
            let due = match call_repository.claim_due_evictions(now, CLAIM_BATCH).await {
                Ok(due) => due,
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to claim pending call evictions");
                    continue;
                }
            };
            for pending in due {
                match call_repository.is_active_participant(pending.call_id, &pending.user_id).await {
                    Ok(true) => {
                        if let Err(e) = call_repository.eviction_done(pending.call_id, &pending.user_id).await {
                            tracing::warn!(error = %e, call_id = %pending.call_id, "Failed to drop a cancelled eviction");
                        }
                        continue;
                    }
                    Ok(false) => {}
                    Err(e) => {
                        tracing::warn!(error = %e, call_id = %pending.call_id, "Failed to revalidate a claimed eviction");
                        continue;
                    }
                }
                let room = ChannelId::new(pending.call_id);
                match voice_service.remove_participant(&room, &pending.user_id).await {
                    Ok(()) => {
                        if let Err(e) = call_repository.eviction_done(pending.call_id, &pending.user_id).await {
                            tracing::warn!(error = %e, call_id = %pending.call_id, "Failed to clear a completed eviction");
                        }
                    }
                    Err(e) => {
                        let backoff = Duration::from_secs((1u64 << pending.attempts.clamp(0, 8) as u32).min(300));
                        let next = now + chrono::Duration::from_std(backoff).unwrap_or(chrono::Duration::seconds(300));
                        tracing::warn!(error = %e, call_id = %pending.call_id, attempts = pending.attempts, "Call eviction retry failed");
                        if let Err(e) = call_repository.eviction_failed(pending.call_id, &pending.user_id, next).await {
                            tracing::warn!(error = %e, call_id = %pending.call_id, "Failed to reschedule a call eviction");
                        }
                    }
                }
            }
        }
    });
}
