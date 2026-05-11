use chrono::{DateTime, Utc};

use crate::{refresh_token::id::RefreshTokenId, user::id::UserId};

#[derive(Debug)]
pub struct RefreshToken {
    id: RefreshTokenId,
    user_id: UserId,
    expires_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl RefreshToken {
    pub fn issue(id: RefreshTokenId, user_id: UserId, expires_at: DateTime<Utc>) -> Self {
        Self {
            id,
            user_id,
            expires_at,
            revoked_at: None,
            created_at: Utc::now(),
        }
    }

    pub fn restore(
        id: RefreshTokenId,
        user_id: UserId,
        expires_at: DateTime<Utc>,
        revoked_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            user_id,
            expires_at,
            revoked_at,
            created_at,
        }
    }

    pub fn is_revoked(&self) -> bool {
        self.revoked_at.is_some()
    }

    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        self.expires_at < now
    }

    pub fn is_active(&self, now: DateTime<Utc>) -> bool {
        !self.is_revoked() && !self.is_expired(now)
    }

    pub fn revoke(&mut self, now: DateTime<Utc>) {
        if self.revoked_at.is_none() {
            self.revoked_at = Some(now);
        }
    }

    pub fn id(&self) -> &RefreshTokenId {
        &self.id
    }

    pub fn user_id(&self) -> &UserId {
        &self.user_id
    }

    pub fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }

    pub fn revoked_at(&self) -> Option<DateTime<Utc>> {
        self.revoked_at
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
