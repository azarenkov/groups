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

#[test]
fn a_deleted_account_cannot_confirm_its_registration() {
    let mut user = fresh_user();
    let code = registration_code(&user);
    user.delete(Utc::now()).unwrap();

    assert!(matches!(
        user.ensure_can_confirm(&code, Utc::now()),
        Err(UserError::Deleted)
    ));
}

#[test]
fn a_deleted_account_cannot_ask_for_another_code() {
    let mut user = fresh_user();
    user.delete(Utc::now()).unwrap();

    assert!(matches!(
        user.ensure_can_request_verification(VerificationPurpose::Registration),
        Err(UserError::Deleted)
    ));
}

#[test]
fn an_expired_plan_counts_as_free() {
    use crate::user::tier::{Entitlements, Tier};
    let now = Utc::now();
    let lapsed = Entitlements { tier: Tier::Pro, valid_until: Some(now - Duration::hours(1)), member_number: Some(1), ..Default::default() };
    let running = Entitlements { tier: Tier::Pro, valid_until: Some(now + Duration::hours(1)), member_number: Some(2), ..Default::default() };
    let open_ended = Entitlements { tier: Tier::Premium, valid_until: None, member_number: Some(3), ..Default::default() };
    assert_eq!(lapsed.active_at(now), Tier::Free);
    assert_eq!(running.active_at(now), Tier::Pro);
    assert_eq!(open_ended.active_at(now), Tier::Premium);
}

#[test]
fn a_trial_is_offered_once_and_only_to_free_accounts() {
    use crate::user::tier::{Entitlements, Tier, TRIAL_DAYS};
    let now = Utc::now();
    assert!(Entitlements::default().trial_available_at(now));

    let on_trial = Entitlements {
        tier: Tier::Pro,
        valid_until: Some(Entitlements::trial_ends_at(now)),
        trial: true,
        trial_used_at: Some(now),
        ..Default::default()
    };
    assert!(on_trial.on_trial_at(now));
    assert!(!on_trial.trial_available_at(now));
    let after = now + Duration::days(TRIAL_DAYS) + Duration::seconds(1);
    assert!(!on_trial.on_trial_at(after));
    assert!(!on_trial.trial_available_at(after));

    let paid = Entitlements { tier: Tier::Pro, valid_until: None, ..Default::default() };
    assert!(!paid.trial_available_at(now));
    assert!(!paid.on_trial_at(now));
}

#[test]
fn trial_email_keys_fold_aliases_of_one_mailbox() {
    use crate::user::tier::trial_email_key;
    assert_eq!(trial_email_key(" Some.One+rift@GMail.com "), "someone@gmail.com");
    assert_eq!(trial_email_key("some.one@googlemail.com"), "someone@gmail.com");
    assert_eq!(trial_email_key("first.last+x@example.com"), "first.last@example.com");
    assert_eq!(trial_email_key("broken"), "broken");
}

#[test]
fn a_badge_style_is_dropped_when_the_plan_no_longer_includes_it() {
    use crate::user::tier::{BadgeStyle, Tier};
    let mut user = fresh_user();
    user.set_badge_style(BadgeStyle::Gold, Utc::now()).unwrap();
    user.pull_events();
    assert!(!user.fit_badge_style_to(Tier::Premium, Utc::now()));
    assert!(user.fit_badge_style_to(Tier::Pro, Utc::now()));
    assert_eq!(user.badge_style(), BadgeStyle::Default);
    assert!(user.pull_events().iter().any(|e| matches!(e, UserEvent::ProfileChanged(_))));

    user.set_badge_style(BadgeStyle::Ink, Utc::now()).unwrap();
    assert!(!user.fit_badge_style_to(Tier::Pro, Utc::now()));
    assert!(user.fit_badge_style_to(Tier::Free, Utc::now()));
}

#[test]
fn a_disabled_account_still_loses_a_badge_its_plan_dropped() {
    use crate::user::tier::{BadgeStyle, Tier};
    let mut user = fresh_user();
    user.set_badge_style(BadgeStyle::Gold, Utc::now()).unwrap();
    user.disable(Utc::now()).unwrap();
    user.pull_events();
    assert!(user.fit_badge_style_to(Tier::Pro, Utc::now()));
    assert_eq!(user.badge_style(), BadgeStyle::Default);
    assert!(!user.pull_events().iter().any(|e| matches!(e, UserEvent::ProfileChanged(_))));
}

#[test]
fn picking_a_badge_style_tells_the_other_services() {
    use crate::user::tier::BadgeStyle;
    let mut user = fresh_user();
    user.set_badge_style(BadgeStyle::Ink, Utc::now()).unwrap();
    assert_eq!(user.badge_style(), BadgeStyle::Ink);
    let events = user.pull_events();
    assert!(events.iter().any(|e| matches!(e, UserEvent::ProfileChanged(p) if p.badge_style == "ink")));

    user.set_badge_style(BadgeStyle::Ink, Utc::now()).unwrap();
    assert!(user.pull_events().is_empty());
    assert!(BadgeStyle::Ink.needs_paid_tier());
    assert!(!BadgeStyle::Default.needs_paid_tier());
    assert_eq!(BadgeStyle::parse("sparkly"), None);
}

fn reset_code(user: &User) -> VerificationCode {
    user.issue_verification(
        VerificationChannel::Email,
        VerificationPurpose::PasswordReset,
        "token".to_string(),
        CodeHash::new("hash".to_string()),
    )
    .unwrap()
}

#[test]
fn a_reset_code_sets_the_new_password_once_and_verifies_the_email() {
    let mut user = fresh_user();
    let mut code = reset_code(&user);
    let now = Utc::now();
    user.reset_password(&mut code, true, PasswordHash::new("new".into()), now).unwrap();
    assert_eq!(user.password_hash().as_str(), "new");
    assert_eq!(user.password_changed_at(), Some(now));
    assert!(user.is_email_verified());
    assert_eq!(code.consumed_at(), Some(now));
    assert!(matches!(
        user.pull_events().as_slice(),
        [UserEvent::EmailVerified(_), UserEvent::PasswordChanged(_)]
    ));
    assert_eq!(
        user.reset_password(&mut code, true, PasswordHash::new("again".into()), now).unwrap_err(),
        UserError::CodeConsumed
    );
    assert_eq!(user.password_hash().as_str(), "new");
}

#[test]
fn a_reset_code_expires_and_locks_after_wrong_secrets() {
    let mut user = fresh_user();
    let mut code = reset_code(&user);
    let later = Utc::now() + Duration::minutes(31);
    assert_eq!(
        user.reset_password(&mut code, true, PasswordHash::new("new".into()), later).unwrap_err(),
        UserError::CodeExpired
    );
    for _ in 0..(VerificationCode::max_attempts() - 1) {
        assert_eq!(
            user.reset_password(&mut code, false, PasswordHash::new("new".into()), Utc::now()).unwrap_err(),
            UserError::BadCode
        );
    }
    assert_eq!(
        user.reset_password(&mut code, false, PasswordHash::new("new".into()), Utc::now()).unwrap_err(),
        UserError::TooManyAttempts
    );
    assert_eq!(
        user.reset_password(&mut code, true, PasswordHash::new("new".into()), Utc::now()).unwrap_err(),
        UserError::TooManyAttempts
    );
    assert_eq!(user.password_hash().as_str(), "hash");
}

#[test]
fn other_codes_cannot_reset_the_password() {
    let mut user = fresh_user();
    let mut code = registration_code(&user);
    assert_eq!(
        user.reset_password(&mut code, true, PasswordHash::new("new".into()), Utc::now()).unwrap_err(),
        UserError::NotPasswordResetCode
    );
}

#[test]
fn a_reset_code_older_than_the_last_password_change_is_rejected() {
    let mut user = fresh_user();
    let mut code = reset_code(&user);
    user.change_password(PasswordHash::new("changed".into()), Utc::now() + Duration::seconds(1));
    assert_eq!(
        user.reset_password(&mut code, true, PasswordHash::new("new".into()), Utc::now()).unwrap_err(),
        UserError::CodeConsumed
    );
    assert_eq!(user.password_hash().as_str(), "changed");
}
