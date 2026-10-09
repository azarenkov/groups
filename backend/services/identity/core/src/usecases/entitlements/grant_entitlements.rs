use std::sync::Arc;

use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use identity_domain::{
    events::IdentityEvent, unit_of_work::IdentityUnitOfWork, user::events::UserEvent,
    user::events::lifecycle::UserEntitlementsChanged, user::tier::Tier,
};
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum GrantEntitlementsError {
    #[error("Unknown tier")]
    UnknownTier,
    #[error("User not found")]
    UserNotFound,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct GrantEntitlements<W: IdentityUnitOfWork> {
    unit_of_work: Arc<W>,
}

impl<W: IdentityUnitOfWork> GrantEntitlements<W> {
    pub fn new(unit_of_work: Arc<W>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        tier: &str,
        valid_until: Option<DateTime<Utc>>,
        source: &str,
    ) -> Result<(), GrantEntitlementsError> {
        let tier = Tier::parse(tier).ok_or(GrantEntitlementsError::UnknownTier)?;
        let mut tx = self.unit_of_work.begin().await?;
        let mut user = tx
            .load_user_for_update(&user_id)
            .await?
            .ok_or(GrantEntitlementsError::UserNotFound)?;
        let now = Utc::now();
        let lapsed_to = tx.load_entitlements(&user_id).await?.active_at(now);
        let mut profile_changed = user.fit_to_tier(lapsed_to, now);
        let member_number = tx
            .save_entitlements(&user_id, tier.as_str(), valid_until, source)
            .await?;
        let mut events = vec![IdentityEvent::from(UserEvent::EntitlementsChanged(
            UserEntitlementsChanged::record(user_id, tier.as_str().to_string(), valid_until, member_number, now),
        ))];
        profile_changed |= user.fit_to_tier(tier, now);
        if profile_changed {
            events.extend(user.pull_events().into_iter().map(IdentityEvent::from));
            tx.save_user(&user).await?;
        }
        tx.save_events(&events, None).await?;
        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "grant_entitlements_tests.rs"]
mod tests;
