pub mod email;
pub mod events;
pub mod id;
pub mod name;
pub mod repository;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{
    shared::value_objects::password_hash::PasswordHash,
    user::{
        email::Email,
        events::{
            UserEvent,
            lifecycle::{UserEmailVerified, UserPhoneVerified, UserRegistered},
        },
        id::UserId,
        name::UserName,
    },
};

#[derive(Debug, Serialize)]
pub struct User {
    id: UserId,
    name: UserName,
    email: Email,
    password_hash: PasswordHash,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    email_verified_at: Option<DateTime<Utc>>,
    phone_verified_at: Option<DateTime<Utc>>,
    #[serde(skip)]
    events: Vec<UserEvent>,
}

impl User {
    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        id: UserId,
        name: UserName,
        email: Email,
        password_hash: PasswordHash,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        email_verified_at: Option<DateTime<Utc>>,
        phone_verified_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            name,
            email,
            password_hash,
            created_at,
            updated_at,
            email_verified_at,
            phone_verified_at,
            events: Vec::new(),
        }
    }

    pub fn create(
        name: UserName,
        email: Email,
        password_hash: PasswordHash,
        ip: Option<String>,
        user_agent: Option<String>,
    ) -> Self {
        let now = Utc::now();
        let id = UserId::generate();
        let event = UserEvent::Registered(UserRegistered::record(
            id,
            name.clone(),
            email.clone(),
            ip,
            user_agent,
            now,
        ));

        Self {
            id,
            name,
            email,
            password_hash,
            created_at: now,
            updated_at: now,
            email_verified_at: None,
            phone_verified_at: None,
            events: vec![event],
        }
    }

    pub fn verify_email(&mut self, now: DateTime<Utc>) {
        self.email_verified_at = Some(now);
        self.record(UserEvent::EmailVerified(UserEmailVerified::record(
            self.id,
            self.email.clone(),
            now,
        )));
    }

    pub fn verify_phone(&mut self, now: DateTime<Utc>) {
        self.phone_verified_at = Some(now);
        self.record(UserEvent::PhoneVerified(UserPhoneVerified::record(
            self.id, now,
        )));
    }

    fn record(&mut self, event: UserEvent) {
        self.events.push(event);
    }

    pub fn pull_events(&mut self) -> Vec<UserEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn is_verified(&self) -> bool {
        self.email_verified_at.is_some() || self.phone_verified_at.is_some()
    }

    pub fn is_email_verified(&self) -> bool {
        self.email_verified_at.is_some()
    }

    pub fn is_phone_verified(&self) -> bool {
        self.phone_verified_at.is_some()
    }

    pub fn id(&self) -> &UserId {
        &self.id
    }

    pub fn id_ownned(&self) -> UserId {
        self.id
    }

    pub fn name(&self) -> &UserName {
        &self.name
    }

    pub fn password_hash(&self) -> &PasswordHash {
        &self.password_hash
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn email(&self) -> &Email {
        &self.email
    }

    pub fn email_verified_at(&self) -> Option<DateTime<Utc>> {
        self.email_verified_at
    }

    pub fn phone_verified_at(&self) -> Option<DateTime<Utc>> {
        self.phone_verified_at
    }
}
