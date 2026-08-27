use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use domain::{shared::outbox::outbox_record::AggregateType, shared::outbox::outbox_record::OutboxRecord, shared::outbox::to_outbox::OutboxError, shared::outbox::to_outbox::ToOutboxRecord};
use crate::{user::events::lifecycle::UserEmailVerified, user::events::lifecycle::UserLoggedIn, user::events::lifecycle::UserLoggedOut, user::events::lifecycle::UserPhoneVerified, user::events::lifecycle::UserRegistered, user::events::lifecycle::UserRenamed, user::id::UserId};

pub mod lifecycle;

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum UserEvent {
    Registered(UserRegistered),
    EmailVerified(UserEmailVerified),
    PhoneVerified(UserPhoneVerified),
    LoggedIn(UserLoggedIn),
    LoggedOut(UserLoggedOut),
    Renamed(UserRenamed),
}

impl UserEvent {
    pub fn user_id(&self) -> &UserId {
        match self {
            Self::Registered(e) => &e.user_id,
            Self::EmailVerified(e) => &e.user_id,
            Self::PhoneVerified(e) => &e.user_id,
            Self::LoggedIn(e) => &e.user_id,
            Self::LoggedOut(e) => &e.user_id,
            Self::Renamed(e) => &e.user_id,
        }
    }

    pub fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            Self::Registered(e) => e.occurred_at,
            Self::EmailVerified(e) => e.occurred_at,
            Self::PhoneVerified(e) => e.occurred_at,
            Self::LoggedIn(e) => e.occurred_at,
            Self::LoggedOut(e) => e.occurred_at,
            Self::Renamed(e) => e.occurred_at,
        }
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Registered(_) => "registered",
            Self::EmailVerified(_) => "email_verified",
            Self::PhoneVerified(_) => "phone_verified",
            Self::LoggedIn(_) => "logged_in",
            Self::LoggedOut(_) => "logged_out",
            Self::Renamed(_) => "renamed",
        }
    }
}

impl ToOutboxRecord for UserEvent {
    fn to_outbox_record(&self) -> Result<OutboxRecord, OutboxError> {
        Ok(OutboxRecord {
            id: Uuid::new_v4(),
            aggregate_id: self.user_id().uuid(),
            aggregate_type: AggregateType::User,
            parent_id: None,
            parent_type: None,
            event_type: self.event_type().to_string(),
            payload: serde_json::to_value(self)?,
            occurred_at: self.occurred_at(),
            request_id: None,
        })
    }
}
