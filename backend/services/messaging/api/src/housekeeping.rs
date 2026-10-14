use std::{sync::Arc, time::Duration};

use messaging::usecases::{community::adopt_legacy_invitations::AdoptLegacyInvitations, inbox::prune_inbox::PruneInbox};
use messaging_domain::{community::invitation::LegacyInvitations, inbox::InboxRepository};

const RUN_EVERY: Duration = Duration::from_secs(60 * 60);

pub fn spawn<R, L>(prune: Arc<PruneInbox<R>>, adopt: Arc<AdoptLegacyInvitations<L>>)
where
    R: InboxRepository + 'static,
    L: LegacyInvitations + 'static,
{
    actix_web::rt::spawn(async move {
        loop {
            match adopt.execute().await {
                Ok(adopted) if adopted > 0 => tracing::info!(adopted, "Adopted legacy DM invitations"),
                Ok(_) => {}
                Err(e) => tracing::warn!(error = %e, "Legacy invitation adoption failed"),
            }
            match prune.execute(chrono::Utc::now()).await {
                Ok(removed) if removed > 0 => tracing::info!(removed, "Pruned inbox notifications"),
                Ok(_) => {}
                Err(e) => tracing::warn!(error = %e, "Inbox prune failed"),
            }
            actix_web::rt::time::sleep(RUN_EVERY).await;
        }
    });
}
