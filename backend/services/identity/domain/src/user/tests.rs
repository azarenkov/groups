use chrono::{Duration, Utc};

use super::{User, UserError, email::Email, id::UserId, locale::Locale, name::UserName};
use crate::{refresh_token::RefreshToken, refresh_token::RefreshTokenError, refresh_token::RevokeOutcome, refresh_token::id::RefreshTokenId, user::events::UserEvent, verification::channel::VerificationChannel, verification::code::VerificationCode, verification::code_hash::CodeHash, verification::purpose::VerificationPurpose};
use domain::password_hash::PasswordHash;

fn fresh_user() -> User {
    let mut user = User::create(
        UserName::new("tester".to_string()),
        Email::new("t@example.com".to_string()),
        PasswordHash::new("hash".to_string()),
        None,
        None,
        Locale::default(),
    );
    user.pull_events();
    user
}

fn registration_code(user: &User) -> VerificationCode {
    user.issue_verification(
        VerificationChannel::Email,
        VerificationPurpose::Registration,
        "123456".to_string(),
        CodeHash::new("hash".to_string()),
    )
    .unwrap()
}

#[test]
fn verified_users_cannot_request_registration_codes() {
    let mut user = fresh_user();
    user.verify_email(Utc::now());
    assert_eq!(
        user.issue_verification(VerificationChannel::Email, VerificationPurpose::Registration, "1".into(), CodeHash::new("h".into()))
            .unwrap_err(),
        UserError::AlreadyVerified
    );
    assert!(user
        .issue_verification(VerificationChannel::Email, VerificationPurpose::PasswordReset, "1".into(), CodeHash::new("h".into()))
        .is_ok());
}

#[test]
fn wrong_codes_count_attempts_and_lock_at_the_limit() {
    let mut user = fresh_user();
    let mut code = registration_code(&user);
    for _ in 0..(VerificationCode::max_attempts() - 1) {
        assert_eq!(user.confirm_registration(&mut code, false, Utc::now()).unwrap_err(), UserError::BadCode);
    }
    assert_eq!(user.confirm_registration(&mut code, false, Utc::now()).unwrap_err(), UserError::TooManyAttempts);
    assert_eq!(code.attempts(), VerificationCode::max_attempts());
    assert_eq!(user.confirm_registration(&mut code, true, Utc::now()).unwrap_err(), UserError::TooManyAttempts);
    assert_eq!(user.ensure_can_confirm(&code, Utc::now()).unwrap_err(), UserError::TooManyAttempts);
    assert!(!user.is_verified());
}

#[test]
fn confirmation_verifies_the_user_and_consumes_the_code_once() {
    let mut user = fresh_user();
    let mut code = registration_code(&user);
    let now = Utc::now();
    user.confirm_registration(&mut code, true, now).unwrap();
    assert!(user.is_email_verified());
    assert_eq!(code.consumed_at(), Some(now));
    assert!(matches!(user.pull_events().as_slice(), [UserEvent::EmailVerified(_)]));
    assert_eq!(user.confirm_registration(&mut code, true, now).unwrap_err(), UserError::AlreadyVerified);
}

#[test]
fn expired_foreign_and_non_registration_codes_are_rejected() {
    let mut user = fresh_user();
    let mut expired = VerificationCode::restore(
        crate::verification::id::VerificationCodeId::generate(),
        *user.id(),
        VerificationChannel::Email,
        CodeHash::new("h".into()),
        VerificationPurpose::Registration,
        Utc::now() - Duration::minutes(1),
        None,
        0,
        Utc::now(),
    );
    assert_eq!(user.confirm_registration(&mut expired, true, Utc::now()).unwrap_err(), UserError::CodeExpired);

    let mut reset = user
        .issue_verification(VerificationChannel::Email, VerificationPurpose::PasswordReset, "1".into(), CodeHash::new("h".into()))
        .unwrap();
    assert_eq!(user.confirm_registration(&mut reset, true, Utc::now()).unwrap_err(), UserError::NotRegistrationCode);

    let other = fresh_user();
    let mut foreign = registration_code(&other);
    assert_eq!(user.confirm_registration(&mut foreign, true, Utc::now()).unwrap_err(), UserError::ForeignCode);
}

#[test]
fn refresh_tokens_only_serve_their_owner() {
    let owner = UserId::generate();
    let stranger = UserId::generate();
    let now = Utc::now();
    let mut token = RefreshToken::issue(RefreshTokenId::generate(), owner, now + Duration::days(1), None, None);
    assert_eq!(token.revoke_by(&stranger, now).unwrap_err(), RefreshTokenError::ForeignToken);
    assert_eq!(token.rotate(&stranger, now).unwrap_err(), RefreshTokenError::ForeignToken);
    token.rotate(&owner, now).unwrap();
    assert!(token.is_revoked());
    assert_eq!(token.rotate(&owner, now).unwrap_err(), RefreshTokenError::Revoked);
    assert_eq!(token.revoke_by(&owner, now).unwrap(), RevokeOutcome::AlreadyRevoked);

    let mut expired = RefreshToken::issue(RefreshTokenId::generate(), owner, now - Duration::days(1), None, None);
    assert_eq!(expired.rotate(&owner, now).unwrap_err(), RefreshTokenError::Expired);
}

#[test]
fn rename_updates_the_name_and_records_an_event() {
    let mut user = fresh_user();
    user.rename(UserName::new("renamed".to_string()), Utc::now()).unwrap();
    assert_eq!(user.name().as_str(), "renamed");
    assert!(matches!(user.pull_events().as_slice(), [UserEvent::Renamed(_)]));
}

#[test]
fn deletion_releases_the_login_and_the_email() {
    let mut user = fresh_user();
    let name_before = user.name().as_str().to_string();
    let email_before = user.email().as_str().to_string();

    user.delete(Utc::now()).unwrap();

    assert_ne!(user.name().as_str(), name_before);
    assert_ne!(user.email().as_str(), email_before);
    assert!(user.is_deleted());
    let events = user.pull_events();
    assert!(events.iter().any(|e| matches!(e, UserEvent::Renamed(_))));
    assert!(events.iter().any(|e| matches!(e, UserEvent::Deleted(_))));
}

#[test]
fn an_account_can_only_be_deleted_once() {
    let mut user = fresh_user();
    user.delete(Utc::now()).unwrap();
    assert!(matches!(user.delete(Utc::now()), Err(UserError::Deleted)));
}
