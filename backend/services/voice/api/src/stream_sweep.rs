use std::sync::Arc;
use std::time::Duration;

use voice::ports::plan_directory::PlanDirectory;
use voice::ports::voice_service::VoiceService;
use voice::ports::webhook_outbox::WebhookOutbox;
use voice::usecases::stream::sweep_screen_shares::SweepScreenShares;

const SWEEP_EVERY: Duration = Duration::from_secs(30);

pub fn spawn<P, V, O>(usecase: Arc<SweepScreenShares<P, V, O>>)
where
    P: PlanDirectory + 'static,
    V: VoiceService + Send + Sync + 'static,
    O: WebhookOutbox + 'static,
{
    actix_web::rt::spawn(async move {
        loop {
            actix_web::rt::time::sleep(SWEEP_EVERY).await;
            if let Err(e) = usecase.execute(chrono::Utc::now()).await {
                tracing::warn!(error = %e, "Screen share sweep failed");
            }
        }
    });
}
