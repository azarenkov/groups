use chrono::{DateTime, Utc};

use crate::{user::events::lifecycle::UserDeleted, user::events::lifecycle::UserDisabled, user::events::lifecycle::UserEmailChanged, user::events::lifecycle::UserEmailVerified, user::events::lifecycle::UserPasswordChanged, user::events::lifecycle::UserReactivated, user::events::lifecycle::UserLoggedIn, user::events::lifecycle::UserLoggedOut, user::events::lifecycle::UserPhoneVerified, user::events::lifecycle::UserRegistered, user::events::lifecycle::UserRenamed, user::id::UserId};

pub mod lifecycle;

#[derive(Debug)]
pub enum UserEvent {
    Registered(UserRegistered),
    EmailVerified(UserEmailVerified),
    PhoneVerified(UserPhoneVerified),
    LoggedIn(UserLoggedIn),
    LoggedOut(UserLoggedOut),
    Renamed(UserRenamed),
    PasswordChanged(UserPasswordChanged),
    EmailChanged(UserEmailChanged),
    Disabled(UserDisabled),
    Reactivated(UserReactivated),
    Deleted(UserDeleted),
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
            Self::PasswordChanged(e) => &e.user_id,
            Self::EmailChanged(e) => &e.user_id,
            Self::Disabled(e) => &e.user_id,
            Self::Reactivated(e) => &e.user_id,
            Self::Deleted(e) => &e.user_id,
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
            Self::PasswordChanged(e) => e.occurred_at,
            Self::EmailChanged(e) => e.occurred_at,
            Self::Disabled(e) => e.occurred_at,
            Self::Reactivated(e) => e.occurred_at,
            Self::Deleted(e) => e.occurred_at,
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
            Self::PasswordChanged(_) => "password_changed",
            Self::EmailChanged(_) => "email_changed",
            Self::Disabled(_) => "disabled",
            Self::Reactivated(_) => "reactivated",
            Self::Deleted(_) => "deleted",
        }
    }
}
