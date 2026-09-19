use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use identity_domain::user::tier::{EntitlementsRepository, Tier};

pub struct ActiveEntitlements {
    pub tier: Tier,
    pub member_number: Option<i64>,
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
        Ok(ActiveEntitlements {
            tier: entitlements.active_at(Utc::now()),
            member_number: entitlements.member_number,
        })
    }
}
