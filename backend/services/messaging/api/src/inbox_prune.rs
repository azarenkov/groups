use std::{sync::Arc, time::Duration};

use messaging::usecases::inbox::prune_inbox::PruneInbox;
use messaging_domain::inbox::InboxRepository;

const PRUNE_EVERY: Duration = Duration::from_secs(60 * 60);

pub fn spawn<R: InboxRepository + 'static>(usecase: Arc<PruneInbox<R>>) {
    actix_web::rt::spawn(async move {
        loop {
            match usecase.execute(chrono::Utc::now()).await {
                Ok(removed) if removed > 0 => tracing::info!(removed, "Pruned inbox notifications"),
                Ok(_) => {}
                Err(e) => tracing::warn!(error = %e, "Inbox prune failed"),
            }
            actix_web::rt::time::sleep(PRUNE_EVERY).await;
        }
    });
}
