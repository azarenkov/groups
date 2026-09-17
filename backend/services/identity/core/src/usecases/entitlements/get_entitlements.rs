use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use identity_domain::user::tier::{EntitlementsRepository, Tier};
use ports::repository::RepositoryError;

pub struct GetEntitlements<E: EntitlementsRepository> {
    entitlements: Arc<E>,
}

impl<E: EntitlementsRepository> GetEntitlements<E> {
    pub fn new(entitlements: Arc<E>) -> Self {
        Self { entitlements }
    }

    pub async fn execute(&self, user_id: &UserId) -> Result<Tier, RepositoryError> {
        Ok(self.entitlements.get(user_id).await?.active_at(Utc::now()))
    }
}
