use std::time::Duration;

use api_common::middleware::revocation::AccessRevocations;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use identity::ports::access_revoker::{AccessRevoker, AccessRevokerError};

const REVOCATION_TTL: Duration = Duration::from_secs(60 * 60);

pub struct RedisAccessRevoker {
    store: AccessRevocations,
}

impl RedisAccessRevoker {
    pub fn new(store: AccessRevocations) -> Self {
        Self { store }
    }
}

#[async_trait]
impl AccessRevoker for RedisAccessRevoker {
    async fn revoke_access_issued_before(&self, user_id: &UserId, cutoff: DateTime<Utc>) -> Result<(), AccessRevokerError> {
        self.store
            .revoke_issued_before(&user_id.uuid(), cutoff.timestamp(), REVOCATION_TTL)
            .await
            .map_err(|e| AccessRevokerError::Store(e.to_string()))
    }
}
