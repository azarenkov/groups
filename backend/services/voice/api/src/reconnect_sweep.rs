use std::sync::Arc;
use std::time::Duration;

use voice::usecases::call::expire_reconnects::ExpireCallReconnects;
use voice_domain::call::repository::CallRepository;

const SWEEP_EVERY: Duration = Duration::from_secs(15);

pub fn spawn<C>(usecase: Arc<ExpireCallReconnects<C>>)
where
    C: CallRepository + 'static,
{
    actix_web::rt::spawn(async move {
        loop {
            actix_web::rt::time::sleep(SWEEP_EVERY).await;
            if let Err(e) = usecase.execute(chrono::Utc::now()).await {
                tracing::warn!(error = %e, "Call reconnect expiry failed");
            }
        }
    });
}
