use std::sync::Arc;

use chrono::Utc;
use identity_domain::{
    events::IdentityEvent, unit_of_work::IdentityUnitOfWork, user::events::UserEvent,
    user::events::lifecycle::UserEntitlementsChanged, user::tier::Tier,
};
use ports::repository::RepositoryError;

pub struct ExpireEntitlements<W: IdentityUnitOfWork> {
    unit_of_work: Arc<W>,
    batch_size: i64,
}

impl<W: IdentityUnitOfWork> ExpireEntitlements<W> {
    pub fn new(unit_of_work: Arc<W>, batch_size: i64) -> Self {
        Self {
            unit_of_work,
            batch_size,
        }
    }

    pub async fn run_once(&self) -> Result<usize, RepositoryError> {
        let now = Utc::now();
        let mut tx = self.unit_of_work.begin().await?;
        let expired = tx
            .load_expired_entitlements_for_update(now, self.batch_size)
            .await?;
        let mut events = Vec::with_capacity(expired.len());
        for user_id in &expired {
            let member_number = tx
                .save_entitlements(user_id, Tier::Free.as_str(), None, "expiry")
                .await?;
            events.push(IdentityEvent::from(UserEvent::EntitlementsChanged(
                UserEntitlementsChanged::record(
                    *user_id,
                    Tier::Free.as_str().to_string(),
                    None,
                    member_number,
                    now,
                ),
            )));
        }
        if !events.is_empty() {
            tx.save_events(&events, None).await?;
        }
        tx.commit().await?;
        Ok(expired.len())
    }
}
