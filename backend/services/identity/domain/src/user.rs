pub use domain::user::{email, id, locale, name};

pub mod events;
pub mod repository;

#[cfg(test)]
mod tests;

use chrono::{DateTime, Utc};
use serde::Serialize;

use domain::password_hash::PasswordHash;
use crate::{user::email::Email, user::events::UserEvent, user::events::lifecycle::UserEmailVerified, user::events::lifecycle::UserPhoneVerified, user::events::lifecycle::UserRegistered, user::events::lifecycle::UserRenamed, user::id::UserId, user::locale::Locale, user::name::UserName, verification::channel::VerificationChannel, verification::code::VerificationCode, verification::code_hash::CodeHash, verification::purpose::VerificationPurpose};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum UserError {
    #[error("User already verified")]
    AlreadyVerified,
    #[error("Verification code not for registration")]
    NotRegistrationCode,
    #[error("Too many attempts for verification code")]
    TooManyAttempts,
    #[error("Verification code has expired")]
    CodeExpired,
    #[error("Verification code already consumed")]
    CodeConsumed,
    #[error("Bad verification code")]
    BadCode,
    #[error("Verification code belongs to another user")]
    ForeignCode,
}

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
    locale: Locale,
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
        locale: Locale,
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
            locale,
            events: Vec::new(),
        }
    }

    pub fn create(
        name: UserName,
        email: Email,
        password_hash: PasswordHash,
        ip: Option<String>,
        user_agent: Option<String>,
        locale: Locale,
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
            locale,
            events: vec![event],
        }
    }

    pub fn issue_verification(
        &self,
        channel: VerificationChannel,
        purpose: VerificationPurpose,
        plain_code: String,
        code_hash: CodeHash,
    ) -> Result<VerificationCode, UserError> {
        if purpose.is_registration() && self.is_verified() {
            return Err(UserError::AlreadyVerified);
        }
        Ok(VerificationCode::issue(
            self.id,
            self.email.clone(),
            plain_code,
            channel,
            code_hash,
            purpose,
            self.locale,
        ))
    }

    pub fn confirm_registration(
        &mut self,
        code: &mut VerificationCode,
        code_matches: bool,
        now: DateTime<Utc>,
    ) -> Result<(), UserError> {
        if self.is_verified() {
            return Err(UserError::AlreadyVerified);
        }
        if *code.user_id() != self.id {
            return Err(UserError::ForeignCode);
        }
        if !code.is_registration_purpose() {
            return Err(UserError::NotRegistrationCode);
        }
        if code.consumed_at().is_some() {
            return Err(UserError::CodeConsumed);
        }
        if code.is_limit_attempts() {
            return Err(UserError::TooManyAttempts);
        }
        if code.expires_at() < now {
            return Err(UserError::CodeExpired);
        }
        if !code_matches {
            code.register_failed_attempt();
            return Err(if code.is_limit_attempts() {
                UserError::TooManyAttempts
            } else {
                UserError::BadCode
            });
        }
        match code.channel() {
            VerificationChannel::Email => self.verify_email(now),
            VerificationChannel::Phone => self.verify_phone(now),
        }
        code.consume(now);
        Ok(())
    }

    pub fn rename(&mut self, name: UserName, now: DateTime<Utc>) {
        self.name = name.clone();
        self.updated_at = now;
        self.record(UserEvent::Renamed(UserRenamed::record(self.id, name, now)));
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

    pub fn set_locale(&mut self, locale: Locale, now: DateTime<Utc>) {
        if self.locale != locale {
            self.locale = locale;
            self.updated_at = now;
        }
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

    pub fn locale(&self) -> Locale {
        self.locale
    }
}
