pub use domain::user::{email, id, locale, name};

pub mod events;
pub mod repository;

#[cfg(test)]
mod tests;

use chrono::{DateTime, Utc};
use serde::Serialize;

use domain::password_hash::PasswordHash;
use crate::{user::email::Email, user::events::UserEvent, user::events::lifecycle::UserDeleted, user::events::lifecycle::UserDisabled, user::events::lifecycle::UserEmailChanged, user::events::lifecycle::UserEmailVerified, user::events::lifecycle::UserPasswordChanged, user::events::lifecycle::UserProfileChanged, user::events::lifecycle::UserReactivated, user::events::lifecycle::UserPhoneVerified, user::events::lifecycle::UserRegistered, user::events::lifecycle::UserRenamed, user::id::UserId, user::locale::Locale, user::name::UserName, verification::channel::VerificationChannel, verification::code::VerificationCode, verification::code_hash::CodeHash, verification::purpose::VerificationPurpose};

pub const BIO_MAX_CHARS: usize = 190;

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
    #[error("Verification code not for email change")]
    NotEmailChangeCode,
    #[error("No pending email change")]
    NoPendingEmail,
    #[error("User is deleted")]
    Deleted,
    #[error("Bio is too long")]
    BioTooLong,
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
    pending_email: Option<Email>,
    password_changed_at: Option<DateTime<Utc>>,
    disabled_at: Option<DateTime<Utc>>,
    deleted_at: Option<DateTime<Utc>>,
    bio: String,
    show_activity: bool,
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
        pending_email: Option<Email>,
        password_changed_at: Option<DateTime<Utc>>,
        disabled_at: Option<DateTime<Utc>>,
        deleted_at: Option<DateTime<Utc>>,
        bio: String,
        show_activity: bool,
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
            pending_email,
            password_changed_at,
            disabled_at,
            deleted_at,
            bio,
            show_activity,
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
            pending_email: None,
            password_changed_at: None,
            disabled_at: None,
            deleted_at: None,
            bio: String::new(),
            show_activity: true,
            events: vec![event],
        }
    }

    pub fn ensure_can_request_verification(&self, purpose: VerificationPurpose) -> Result<(), UserError> {
        if purpose.is_registration() && self.is_verified() {
            return Err(UserError::AlreadyVerified);
        }
        Ok(())
    }

    pub fn issue_verification(
        &self,
        channel: VerificationChannel,
        purpose: VerificationPurpose,
        plain_code: String,
        code_hash: CodeHash,
    ) -> Result<VerificationCode, UserError> {
        self.ensure_can_request_verification(purpose)?;
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

    pub fn ensure_can_confirm(&self, code: &VerificationCode, now: DateTime<Utc>) -> Result<(), UserError> {
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
        Ok(())
    }

    pub fn confirm_registration(
        &mut self,
        code: &mut VerificationCode,
        code_matches: bool,
        now: DateTime<Utc>,
    ) -> Result<(), UserError> {
        self.ensure_can_confirm(code, now)?;
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

    pub fn change_password(&mut self, password_hash: PasswordHash, now: DateTime<Utc>) {
        self.password_hash = password_hash;
        self.password_changed_at = Some(now);
        self.updated_at = now;
        self.record(UserEvent::PasswordChanged(UserPasswordChanged::record(self.id, now)));
    }

    pub fn request_email_change(
        &mut self,
        new_email: Email,
        plain_code: String,
        code_hash: CodeHash,
        now: DateTime<Utc>,
    ) -> Result<VerificationCode, UserError> {
        if self.deleted_at.is_some() {
            return Err(UserError::Deleted);
        }
        self.pending_email = Some(new_email.clone());
        self.updated_at = now;
        Ok(VerificationCode::issue(
            self.id,
            new_email,
            plain_code,
            VerificationChannel::Email,
            code_hash,
            VerificationPurpose::EmailChange,
            self.locale,
        ))
    }

    pub fn ensure_can_confirm_email_change(&self, code: &VerificationCode, now: DateTime<Utc>) -> Result<(), UserError> {
        if self.deleted_at.is_some() {
            return Err(UserError::Deleted);
        }
        if self.pending_email.is_none() {
            return Err(UserError::NoPendingEmail);
        }
        if *code.user_id() != self.id {
            return Err(UserError::ForeignCode);
        }
        if !matches!(code.purpose(), VerificationPurpose::EmailChange) {
            return Err(UserError::NotEmailChangeCode);
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
        Ok(())
    }

    pub fn confirm_email_change(
        &mut self,
        code: &mut VerificationCode,
        code_matches: bool,
        now: DateTime<Utc>,
    ) -> Result<(), UserError> {
        self.ensure_can_confirm_email_change(code, now)?;
        if !code_matches {
            code.register_failed_attempt();
            return Err(if code.is_limit_attempts() {
                UserError::TooManyAttempts
            } else {
                UserError::BadCode
            });
        }
        let email = self.pending_email.take().ok_or(UserError::NoPendingEmail)?;
        self.email = email.clone();
        self.email_verified_at = Some(now);
        self.updated_at = now;
        code.consume(now);
        self.record(UserEvent::EmailChanged(UserEmailChanged::record(self.id, email, now)));
        Ok(())
    }

    pub fn disable(&mut self, now: DateTime<Utc>) -> Result<(), UserError> {
        if self.deleted_at.is_some() {
            return Err(UserError::Deleted);
        }
        if self.disabled_at.is_none() {
            self.disabled_at = Some(now);
            self.updated_at = now;
            self.record(UserEvent::Disabled(UserDisabled::record(self.id, now)));
            self.record(UserEvent::ProfileChanged(UserProfileChanged::record(self.id, String::new(), false, false, now)));
        }
        Ok(())
    }

    pub fn reactivate(&mut self, now: DateTime<Utc>) {
        if self.disabled_at.is_some() {
            self.disabled_at = None;
            self.updated_at = now;
            self.record(UserEvent::Reactivated(UserReactivated::record(self.id, now)));
            self.record(UserEvent::ProfileChanged(UserProfileChanged::record(
                self.id,
                self.bio.clone(),
                self.show_activity,
                true,
                now,
            )));
        }
    }

    pub fn delete(&mut self, now: DateTime<Utc>) -> Result<(), UserError> {
        if self.deleted_at.is_some() {
            return Err(UserError::Deleted);
        }
        let name = UserName::new(format!("deleted_{}", self.id.uuid().simple()));
        self.name = name.clone();
        self.deleted_at = Some(now);
        let id = self.id.uuid().simple().to_string();
        let nonce = UserId::generate().uuid().simple().to_string();
        self.email = Email::new(format!("deleted-{id}-{nonce}-{nonce}@deleted.invalid"));
        self.pending_email = None;
        self.bio = String::new();
        self.show_activity = false;
        self.updated_at = now;
        self.record(UserEvent::Renamed(UserRenamed::record(self.id, name, now)));
        self.record(UserEvent::ProfileChanged(UserProfileChanged::record(self.id, String::new(), false, false, now)));
        self.record(UserEvent::Deleted(UserDeleted::record(self.id, now)));
        Ok(())
    }

    pub fn update_profile(&mut self, bio: String, show_activity: bool, now: DateTime<Utc>) -> Result<(), UserError> {
        if self.deleted_at.is_some() || self.disabled_at.is_some() {
            return Err(UserError::Deleted);
        }
        let bio = bio.trim().to_string();
        if bio.chars().count() > BIO_MAX_CHARS {
            return Err(UserError::BioTooLong);
        }
        if self.bio == bio && self.show_activity == show_activity {
            return Ok(());
        }
        self.bio = bio.clone();
        self.show_activity = show_activity;
        self.updated_at = now;
        self.record(UserEvent::ProfileChanged(UserProfileChanged::record(self.id, bio, show_activity, true, now)));
        Ok(())
    }

    pub fn bio(&self) -> &str {
        &self.bio
    }

    pub fn show_activity(&self) -> bool {
        self.show_activity
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled_at.is_some()
    }

    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    pub fn pending_email(&self) -> Option<&Email> {
        self.pending_email.as_ref()
    }

    pub fn password_changed_at(&self) -> Option<DateTime<Utc>> {
        self.password_changed_at
    }

    pub fn disabled_at(&self) -> Option<DateTime<Utc>> {
        self.disabled_at
    }

    pub fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }

    pub fn rename(&mut self, name: UserName, now: DateTime<Utc>) -> Result<(), UserError> {
        if self.deleted_at.is_some() {
            return Err(UserError::Deleted);
        }
        self.name = name.clone();
        self.updated_at = now;
        self.record(UserEvent::Renamed(UserRenamed::record(self.id, name, now)));
        Ok(())
    }

    pub fn claim_with_oauth(&mut self, password_hash: PasswordHash, now: DateTime<Utc>) {
        self.password_hash = password_hash;
        self.updated_at = now;
        if self.email_verified_at.is_none() {
            self.verify_email(now);
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
