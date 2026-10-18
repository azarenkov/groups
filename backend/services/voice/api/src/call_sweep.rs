use std::sync::Arc;
use std::time::Duration;

use voice::ports::voice_service::VoiceService;
use voice::usecases::call::{
    expire_invitations::ExpireCallInvitations, expire_reconnects::ExpireCallReconnects, reconcile_sessions::ReconcileCallSessions,
};
use voice_domain::call::repository::CallRepository;

const EXPIRY_EVERY: Duration = Duration::from_secs(15);
const RECONCILE_EVERY: Duration = Duration::from_secs(30);

pub fn spawn<C, V>(call_repository: Arc<C>, voice_service: Arc<V>)
where
    C: CallRepository + Send + Sync + 'static,
    V: VoiceService + Send + Sync + 'static,
{
    let reconnects = ExpireCallReconnects::new(call_repository.clone());
    actix_web::rt::spawn(async move {
        loop {
            actix_web::rt::time::sleep(EXPIRY_EVERY).await;
            if let Err(e) = reconnects.execute(chrono::Utc::now()).await {
                tracing::warn!(error = %e, "Call reconnect expiry failed");
            }
        }
    });

    let invitations = ExpireCallInvitations::new(call_repository.clone());
    actix_web::rt::spawn(async move {
        loop {
            actix_web::rt::time::sleep(EXPIRY_EVERY).await;
            if let Err(e) = invitations.execute(chrono::Utc::now()).await {
                tracing::warn!(error = %e, "Call invitation expiry failed");
            }
        }
    });

    let sessions = ReconcileCallSessions::new(call_repository, voice_service);
    actix_web::rt::spawn(async move {
        loop {
            actix_web::rt::time::sleep(RECONCILE_EVERY).await;
            match sessions.execute(chrono::Utc::now()).await {
                Ok(summary) if summary.dropped > 0 || summary.livekit_errors > 0 => {
                    tracing::info!(
                        calls_checked = summary.calls_checked,
                        sessions_checked = summary.sessions_checked,
                        dropped = summary.dropped,
                        livekit_errors = summary.livekit_errors,
                        "Call session reconciliation pass completed",
                    );
                }
                Ok(_) => {}
                Err(e) => tracing::warn!(error = %e, "Call session reconciliation failed"),
            }
        }
    });
}
