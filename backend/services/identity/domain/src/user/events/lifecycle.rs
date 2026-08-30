use chrono::{DateTime, Utc};

use crate::{refresh_token::id::RefreshTokenId, user::email::Email, user::id::UserId, user::name::UserName};
use domain::event::EventId;

#[derive(Debug)]
pub struct UserRegistered {
    pub id: EventId,
    pub user_id: UserId,
    pub name: UserName,
    pub email: Email,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

impl UserRegistered {
    pub fn record(
        user_id: UserId,
        name: UserName,
        email: Email,
        ip: Option<String>,
        user_agent: Option<String>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EventId::generate(),
            user_id,
            name,
            email,
            ip,
            user_agent,
            occurred_at: now,
        }
    }
}

#[derive(Debug)]
pub struct UserEmailVerified {
    pub id: EventId,
    pub user_id: UserId,
    pub email: Email,
    pub occurred_at: DateTime<Utc>,
}

impl UserEmailVerified {
    pub fn record(user_id: UserId, email: Email, now: DateTime<Utc>) -> Self {
        Self {
            id: EventId::generate(),
            user_id,
            email,
            occurred_at: now,
        }
    }
}

#[derive(Debug)]
pub struct UserPhoneVerified {
    pub id: EventId,
    pub user_id: UserId,
    pub occurred_at: DateTime<Utc>,
}

impl UserPhoneVerified {
    pub fn record(user_id: UserId, now: DateTime<Utc>) -> Self {
        Self {
            id: EventId::generate(),
            user_id,
            occurred_at: now,
        }
    }
}

#[derive(Debug)]
pub struct UserLoggedIn {
    pub id: EventId,
    pub user_id: UserId,
    pub refresh_token_id: RefreshTokenId,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

impl UserLoggedIn {
    pub fn record(
        user_id: UserId,
        refresh_token_id: RefreshTokenId,
        ip: Option<String>,
        user_agent: Option<String>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EventId::generate(),
            user_id,
            refresh_token_id,
            ip,
            user_agent,
            occurred_at: now,
        }
    }
}

#[derive(Debug)]
pub struct UserLoggedOut {
    pub id: EventId,
    pub user_id: UserId,
    pub refresh_token_id: RefreshTokenId,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

impl UserLoggedOut {
    pub fn record(
        user_id: UserId,
        refresh_token_id: RefreshTokenId,
        ip: Option<String>,
        user_agent: Option<String>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EventId::generate(),
            user_id,
            refresh_token_id,
            ip,
            user_agent,
            occurred_at: now,
        }
    }
}

#[derive(Debug)]
pub struct UserRenamed {
    pub id: EventId,
    pub user_id: UserId,
    pub name: UserName,
    pub occurred_at: DateTime<Utc>,
}

impl UserRenamed {
    pub fn record(user_id: UserId, name: UserName, now: DateTime<Utc>) -> Self {
        Self {
            id: EventId::generate(),
            user_id,
            name,
            occurred_at: now,
        }
    }
}

#[derive(Debug)]
pub struct UserPasswordChanged {
    pub id: EventId,
    pub user_id: UserId,
    pub occurred_at: DateTime<Utc>,
}

impl UserPasswordChanged {
    pub fn record(user_id: UserId, now: DateTime<Utc>) -> Self {
        Self {
            id: EventId::generate(),
            user_id,
            occurred_at: now,
        }
    }
}

#[derive(Debug)]
pub struct UserEmailChanged {
    pub id: EventId,
    pub user_id: UserId,
    pub email: Email,
    pub occurred_at: DateTime<Utc>,
}

impl UserEmailChanged {
    pub fn record(user_id: UserId, email: Email, now: DateTime<Utc>) -> Self {
        Self {
            id: EventId::generate(),
            user_id,
            email,
            occurred_at: now,
        }
    }
}

#[derive(Debug)]
pub struct UserDisabled {
    pub id: EventId,
    pub user_id: UserId,
    pub occurred_at: DateTime<Utc>,
}

impl UserDisabled {
    pub fn record(user_id: UserId, now: DateTime<Utc>) -> Self {
        Self {
            id: EventId::generate(),
            user_id,
            occurred_at: now,
        }
    }
}

#[derive(Debug)]
pub struct UserReactivated {
    pub id: EventId,
    pub user_id: UserId,
    pub occurred_at: DateTime<Utc>,
}

impl UserReactivated {
    pub fn record(user_id: UserId, now: DateTime<Utc>) -> Self {
        Self {
            id: EventId::generate(),
            user_id,
            occurred_at: now,
        }
    }
}

#[derive(Debug)]
pub struct UserDeleted {
    pub id: EventId,
    pub user_id: UserId,
    pub occurred_at: DateTime<Utc>,
}

impl UserDeleted {
    pub fn record(user_id: UserId, now: DateTime<Utc>) -> Self {
        Self {
            id: EventId::generate(),
            user_id,
            occurred_at: now,
        }
    }
}

#[derive(Debug)]
pub struct UserProfileChanged {
    pub id: EventId,
    pub user_id: UserId,
    pub bio: String,
    pub show_activity: bool,
    pub active: bool,
    pub timezone: String,
    pub occurred_at: DateTime<Utc>,
}

impl UserProfileChanged {
    pub fn record(user_id: UserId, bio: String, show_activity: bool, active: bool, timezone: String, now: DateTime<Utc>) -> Self {
        Self {
            id: EventId::generate(),
            user_id,
            bio,
            show_activity,
            active,
            timezone,
            occurred_at: now,
        }
    }
}
