pub mod id;

use chrono::{DateTime, Utc};

use crate::{refresh_token::id::RefreshTokenId, user::id::UserId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RefreshTokenError {
    #[error("Refresh token does not belong to user")]
    ForeignToken,
    #[error("Refresh token has been revoked")]
    Revoked,
    #[error("Refresh token has expired")]
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevokeOutcome {
    Revoked,
    AlreadyRevoked,
}

#[derive(Debug)]
pub struct RefreshToken {
    id: RefreshTokenId,
    user_id: UserId,
    expires_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    last_used_at: DateTime<Utc>,
    last_ip: Option<String>,
    last_user_agent: Option<String>,
}

impl RefreshToken {
    pub fn issue(
        id: RefreshTokenId,
        user_id: UserId,
        expires_at: DateTime<Utc>,
        ip: Option<String>,
        user_agent: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            user_id,
            expires_at,
            revoked_at: None,
            created_at: now,
            last_used_at: now,
            last_ip: ip,
            last_user_agent: user_agent,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        id: RefreshTokenId,
        user_id: UserId,
        expires_at: DateTime<Utc>,
        revoked_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
        last_used_at: DateTime<Utc>,
        last_ip: Option<String>,
        last_user_agent: Option<String>,
    ) -> Self {
        Self {
            id,
            user_id,
            expires_at,
            revoked_at,
            created_at,
            last_used_at,
            last_ip,
            last_user_agent,
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

    pub fn revoke_by(&mut self, user: &UserId, now: DateTime<Utc>) -> Result<RevokeOutcome, RefreshTokenError> {
        if self.user_id != *user {
            return Err(RefreshTokenError::ForeignToken);
        }
        if self.revoked_at.is_some() {
            return Ok(RevokeOutcome::AlreadyRevoked);
        }
        self.revoked_at = Some(now);
        Ok(RevokeOutcome::Revoked)
    }

    pub fn rotate(&mut self, user: &UserId, now: DateTime<Utc>) -> Result<(), RefreshTokenError> {
        if self.user_id != *user {
            return Err(RefreshTokenError::ForeignToken);
        }
        if self.is_revoked() {
            return Err(RefreshTokenError::Revoked);
        }
        if self.is_expired(now) {
            return Err(RefreshTokenError::Expired);
        }
        self.revoked_at = Some(now);
        Ok(())
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

    pub fn last_used_at(&self) -> DateTime<Utc> {
        self.last_used_at
    }

    pub fn last_ip(&self) -> Option<&str> {
        self.last_ip.as_deref()
    }

    pub fn last_user_agent(&self) -> Option<&str> {
        self.last_user_agent.as_deref()
    }
}
