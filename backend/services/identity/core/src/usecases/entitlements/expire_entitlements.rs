use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
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

    pub async fn drain(&self) -> Result<usize, RepositoryError> {
        let mut total = 0;
        loop {
            let (seen, expired) = self.run_once().await?;
            total += expired;
            if (seen as i64) < self.batch_size || expired == 0 {
                return Ok(total);
            }
        }
    }

    pub async fn run_once(&self) -> Result<(usize, usize), RepositoryError> {
        let candidates = {
            let mut tx = self.unit_of_work.begin().await?;
            let ids = tx.expired_entitlement_users(Utc::now(), self.batch_size).await?;
            tx.commit().await?;
            ids
        };
        let mut expired = 0;
        for user_id in &candidates {
            if self.expire(user_id).await? {
                expired += 1;
            }
        }
        Ok((candidates.len(), expired))
    }

    async fn expire(&self, user_id: &UserId) -> Result<bool, RepositoryError> {
        let mut tx = self.unit_of_work.begin().await?;
        let mut user = tx.load_user_for_update(user_id).await?;
        let now = Utc::now();
        let entitlements = tx.load_entitlements(user_id).await?;
        let lapsed = entitlements.tier != Tier::Free && entitlements.valid_until.is_some_and(|until| until <= now);
        if !lapsed {
            return Ok(false);
        }
        let member_number = tx
            .save_entitlements(user_id, Tier::Free.as_str(), None, "expiry")
            .await?;
        let mut events = vec![IdentityEvent::from(UserEvent::EntitlementsChanged(
            UserEntitlementsChanged::record(*user_id, Tier::Free.as_str().to_string(), None, member_number, now),
        ))];
        if let Some(user) = user.as_mut()
            && user.fit_to_tier(Tier::Free, now)
        {
            events.extend(user.pull_events().into_iter().map(IdentityEvent::from));
            tx.save_user(user).await?;
        }
        tx.save_events(&events, None).await?;
        tx.commit().await?;
        Ok(true)
    }
}

#[cfg(test)]
#[path = "expire_entitlements_tests.rs"]
mod tests;
