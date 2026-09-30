use std::sync::Arc;

use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use identity_domain::user::tier::{EntitlementsRepository, Tier};

pub struct ActiveEntitlements {
    pub tier: Tier,
    pub member_number: Option<i64>,
    pub valid_until: Option<DateTime<Utc>>,
    pub trial: bool,
    pub trial_available: bool,
}
use ports::repository::RepositoryError;

pub struct GetEntitlements<E: EntitlementsRepository> {
    entitlements: Arc<E>,
}

impl<E: EntitlementsRepository> GetEntitlements<E> {
    pub fn new(entitlements: Arc<E>) -> Self {
        Self { entitlements }
    }

    pub async fn execute(&self, user_id: &UserId) -> Result<ActiveEntitlements, RepositoryError> {
        let entitlements = self.entitlements.get(user_id).await?;
        let now = Utc::now();
        let tier = entitlements.active_at(now);
        Ok(ActiveEntitlements {
            tier,
            member_number: entitlements.member_number,
            valid_until: entitlements.valid_until.filter(|_| tier.is_paid()),
            trial: entitlements.on_trial_at(now),
            trial_available: entitlements.trial_available_at(now),
        })
    }
}
