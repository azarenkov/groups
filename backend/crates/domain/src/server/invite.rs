use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{server::id::ServerId, user::id::UserId};

#[derive(Debug, Clone)]
pub struct ServerInvite {
    pub id: Uuid,
    pub server_id: ServerId,
    pub code: String,
    pub created_by: UserId,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

impl ServerInvite {
    pub fn create(server_id: ServerId, created_by: UserId, now: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4(),
            server_id,
            code: generate_code(),
            created_by,
            created_at: now,
            revoked_at: None,
        }
    }

    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none()
    }
}

pub fn generate_code() -> String {
    Uuid::new_v4().simple().to_string()[..8].to_string()
}
