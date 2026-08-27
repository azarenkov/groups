use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{refresh_token::id::RefreshTokenId, user::email::Email, user::id::UserId, user::name::UserName};
use domain::shared::event::event_id::EventId;

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
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
