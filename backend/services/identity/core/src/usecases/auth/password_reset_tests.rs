use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use domain::{password_hash::PasswordHash, user::email::Email, user::id::UserId, user::locale::Locale, user::name::UserName};
use identity_domain::{
    events::IdentityEvent,
    refresh_token::{RefreshToken, id::RefreshTokenId},
    unit_of_work::{IdentityTransaction, IdentityUnitOfWork},
    user::{User, repository::UserRepository, tier::{Entitlements, TrialHistory}},
    user::settings::{appearance::AppearanceSettings, hotkeys::HotkeyBindings, notification::NotificationSettings, voice::VoiceSettings},
    verification::{channel::VerificationChannel, code::VerificationCode, code_hash::CodeHash, id::VerificationCodeId, purpose::VerificationPurpose, throttle::IssuedCodes},
};
use ports::repository::RepositoryError;

use super::*;
use crate::ports::{access_revoker::{AccessRevoker, AccessRevokerError}, hasher::{HashError, Hasher}};

struct StoredCode {
    id: VerificationCodeId,
    user_id: UserId,
    hash: String,
    purpose: VerificationPurpose,
    expires_at: DateTime<Utc>,
    consumed_at: Option<DateTime<Utc>>,
    attempts: u8,
    created_at: DateTime<Utc>,
    plain: Option<String>,
}

#[derive(Default)]
struct State {
    user: Option<User>,
    codes: Vec<StoredCode>,
    refresh_revoked_at: Option<DateTime<Utc>>,
    access_revoked_before: Option<DateTime<Utc>>,
}

#[derive(Clone, Default)]
struct Fake(Arc<Mutex<State>>);

fn copy_user(user: &User) -> User {
    User::restore(
        *user.id(),
        user.name().clone(),
        user.email().clone(),
        PasswordHash::new(user.password_hash().as_str().to_string()),
        user.created_at(),
        user.updated_at(),
        user.email_verified_at(),
        user.phone_verified_at(),
        user.locale(),
        user.pending_email().cloned(),
        user.password_changed_at(),
        user.disabled_at(),
        user.deleted_at(),
        user.bio().to_string(),
        user.badge_style(),
        user.show_activity(),
        user.show_game_activity(),
        user.timezone().to_string(),
        user.emoji_status(),
    )
}

fn restore_code(code: &StoredCode) -> VerificationCode {
    VerificationCode::restore(
        code.id,
        code.user_id,
        VerificationChannel::Email,
        CodeHash::new(code.hash.clone()),
        code.purpose,
        code.expires_at,
        code.consumed_at,
        code.attempts,
        code.created_at,
    )
}

impl Fake {
    fn with_user() -> (Self, UserId) {
        Self::with_email("t@example.com")
    }

    fn with_email(email: &str) -> (Self, UserId) {
        let mut user = User::create(
            UserName::new("tester".to_string()),
            Email::new(email.to_string()),
            PasswordHash::new("h:old".to_string()),
            None,
            None,
            Locale::default(),
        );
        user.verify_email(Utc::now());
        user.pull_events();
        let id = *user.id();
        let fake = Self::default();
        fake.0.lock().unwrap().user = Some(user);
        (fake, id)
    }

    fn tokens(&self) -> Vec<String> {
        self.0.lock().unwrap().codes.iter().filter_map(|c| c.plain.clone()).collect()
    }

    fn password(&self) -> String {
        self.0.lock().unwrap().user.as_ref().unwrap().password_hash().as_str().to_string()
    }

    fn age_codes(&self, by: Duration) {
        let mut state = self.0.lock().unwrap();
        for code in &mut state.codes {
            code.created_at -= by;
            code.expires_at -= by;
        }
    }
}

#[async_trait]
impl IdentityUnitOfWork for Fake {
    async fn begin(&self) -> Result<Box<dyn IdentityTransaction>, RepositoryError> {
        Ok(Box::new(self.clone()))
    }
}

#[async_trait]
impl IdentityTransaction for Fake {
    async fn load_user_for_update(&mut self, user_id: &UserId) -> Result<Option<User>, RepositoryError> {
        Ok(self.0.lock().unwrap().user.as_ref().filter(|u| u.id() == user_id).map(copy_user))
    }
    async fn load_pending_verification_for_purpose_for_update(&mut self, user_id: &UserId, purpose: VerificationPurpose) -> Result<Option<VerificationCode>, RepositoryError> {
        let state = self.0.lock().unwrap();
        Ok(state
            .codes
            .iter()
            .filter(|c| c.user_id == *user_id && c.consumed_at.is_none() && std::mem::discriminant(&c.purpose) == std::mem::discriminant(&purpose))
            .max_by_key(|c| c.created_at)
            .map(restore_code))
    }
    async fn revoke_refresh_tokens_for_user(&mut self, _user_id: &UserId, _keep: Option<&RefreshTokenId>, now: DateTime<Utc>) -> Result<(), RepositoryError> {
        self.0.lock().unwrap().refresh_revoked_at = Some(now);
        Ok(())
    }
    async fn save_user(&mut self, user: &User) -> Result<(), RepositoryError> {
        self.0.lock().unwrap().user = Some(copy_user(user));
        Ok(())
    }
    async fn insert_verification_code(&mut self, code: &VerificationCode) -> Result<(), RepositoryError> {
        let mut state = self.0.lock().unwrap();
        state.codes.push(StoredCode {
            id: *code.id(),
            user_id: *code.user_id(),
            hash: code.code_hash().as_str().to_string(),
            purpose: *code.purpose(),
            expires_at: code.expires_at(),
            consumed_at: code.consumed_at(),
            attempts: code.attempts(),
            created_at: code.created_at(),
            plain: None,
        });
        Ok(())
    }
    async fn save_verification_code(&mut self, code: &VerificationCode) -> Result<(), RepositoryError> {
        let mut state = self.0.lock().unwrap();
        let stored = state.codes.iter_mut().find(|c| c.id.uuid() == code.id().uuid()).ok_or(RepositoryError::NotFound)?;
        stored.attempts = code.attempts();
        stored.consumed_at = code.consumed_at();
        Ok(())
    }
    async fn verification_code_owner(&mut self, id: &VerificationCodeId) -> Result<Option<UserId>, RepositoryError> {
        Ok(self.0.lock().unwrap().codes.iter().find(|c| c.id.uuid() == id.uuid()).map(|c| c.user_id))
    }
    async fn load_verification_code_for_update(&mut self, id: &VerificationCodeId) -> Result<Option<VerificationCode>, RepositoryError> {
        Ok(self.0.lock().unwrap().codes.iter().find(|c| c.id.uuid() == id.uuid()).map(restore_code))
    }
    async fn consume_pending_verifications(&mut self, user_id: &UserId, purpose: VerificationPurpose, now: DateTime<Utc>) -> Result<(), RepositoryError> {
        let mut state = self.0.lock().unwrap();
        for code in state.codes.iter_mut().filter(|c| c.user_id == *user_id && std::mem::discriminant(&c.purpose) == std::mem::discriminant(&purpose)) {
            code.consumed_at.get_or_insert(now);
        }
        Ok(())
    }
    async fn save_events(&mut self, events: &[IdentityEvent], _request_id: Option<&str>) -> Result<(), RepositoryError> {
        let mut state = self.0.lock().unwrap();
        for event in events {
            if let IdentityEvent::Verification(identity_domain::verification::events::VerificationEvent::Issued(issued)) = event
                && let Some(code) = state.codes.iter_mut().find(|c| c.id.uuid() == issued.verification_code_id.uuid())
            {
                code.plain = Some(issued.code.clone());
            }
        }
        Ok(())
    }
    async fn issued_verification_codes(&mut self, user_id: &UserId, _purpose: VerificationPurpose, since: DateTime<Utc>) -> Result<IssuedCodes, RepositoryError> {
        let state = self.0.lock().unwrap();
        let recent: Vec<_> = state.codes.iter().filter(|c| c.user_id == *user_id && c.created_at >= since).map(|c| c.created_at).collect();
        Ok(IssuedCodes {
            last_issued_at: recent.iter().max().copied(),
            oldest_in_window: recent.iter().min().copied(),
            issued_last_hour: recent.len() as i64,
        })
    }
    async fn commit(self: Box<Self>) -> Result<(), RepositoryError> {
        Ok(())
    }

    async fn load_stale_unverified_users_for_update(&mut self, _: DateTime<Utc>, _: i64) -> Result<Vec<User>, RepositoryError> { unimplemented!() }
    async fn load_pending_verification_for_update(&mut self, _: &UserId) -> Result<Option<VerificationCode>, RepositoryError> { unimplemented!() }
    async fn delete_oauth_identities(&mut self, _: &UserId) -> Result<(), RepositoryError> { unimplemented!() }
    async fn load_refresh_token_for_update(&mut self, _: &RefreshTokenId) -> Result<Option<RefreshToken>, RepositoryError> { unimplemented!() }
    async fn insert_user(&mut self, _: &User) -> Result<(), RepositoryError> { unimplemented!() }
    async fn insert_refresh_token(&mut self, _: &RefreshToken) -> Result<(), RepositoryError> { Ok(()) }
    async fn save_refresh_token(&mut self, _: &RefreshToken) -> Result<(), RepositoryError> { unimplemented!() }
    async fn lock_user(&mut self, _: &UserId) -> Result<bool, RepositoryError> { unimplemented!() }
    async fn load_entitlements(&mut self, _: &UserId) -> Result<Entitlements, RepositoryError> { unimplemented!() }
    async fn save_entitlements(&mut self, _: &UserId, _: &str, _: Option<DateTime<Utc>>, _: &str) -> Result<Option<i64>, RepositoryError> { unimplemented!() }
    async fn trial_history(&mut self, _: &Email, _: Option<&str>, _: DateTime<Utc>) -> Result<TrialHistory, RepositoryError> { unimplemented!() }
    async fn start_trial(&mut self, _: &UserId, _: &Email, _: Option<&str>, _: DateTime<Utc>, _: DateTime<Utc>) -> Result<(), RepositoryError> { unimplemented!() }
    async fn load_trials_ending_for_update(&mut self, _: DateTime<Utc>, _: i64) -> Result<Vec<UserId>, RepositoryError> { unimplemented!() }
    async fn mark_trial_reminded(&mut self, _: &UserId, _: DateTime<Utc>) -> Result<(), RepositoryError> { unimplemented!() }
    async fn expired_entitlement_users(&mut self, _: DateTime<Utc>, _: i64) -> Result<Vec<UserId>, RepositoryError> { unimplemented!() }
    async fn load_appearance_for_update(&mut self, _: &UserId) -> Result<Option<AppearanceSettings>, RepositoryError> { unimplemented!() }
    async fn save_appearance(&mut self, _: &UserId, _: &AppearanceSettings) -> Result<(), RepositoryError> { unimplemented!() }
    async fn load_voice_settings_for_update(&mut self, _: &UserId) -> Result<Option<VoiceSettings>, RepositoryError> { unimplemented!() }
    async fn save_voice_settings(&mut self, _: &UserId, _: &VoiceSettings) -> Result<(), RepositoryError> { unimplemented!() }
    async fn load_notification_settings_for_update(&mut self, _: &UserId) -> Result<Option<NotificationSettings>, RepositoryError> { unimplemented!() }
    async fn save_notification_settings(&mut self, _: &UserId, _: &NotificationSettings) -> Result<(), RepositoryError> { unimplemented!() }
    async fn load_hotkey_settings_for_update(&mut self, _: &UserId) -> Result<Option<HotkeyBindings>, RepositoryError> { unimplemented!() }
    async fn save_hotkey_settings(&mut self, _: &UserId, _: &HotkeyBindings) -> Result<(), RepositoryError> { unimplemented!() }
    async fn load_presence_preference_for_update(&mut self, _: &UserId) -> Result<Option<identity_domain::user::presence::PresencePreference>, RepositoryError> { unimplemented!() }
    async fn save_presence_preference(&mut self, _: &identity_domain::user::presence::PresencePreference) -> Result<(), RepositoryError> { unimplemented!() }
}

#[async_trait]
impl UserRepository for Fake {
    async fn appear_offline_among(&self, _: &[UserId]) -> Result<Vec<UserId>, RepositoryError> { unimplemented!() }
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepositoryError> {
        Ok(self.0.lock().unwrap().user.as_ref().filter(|u| u.email().as_str() == email.as_str()).map(copy_user))
    }
    async fn find_by_email_ignoring_case(&self, email: &Email) -> Result<Option<User>, RepositoryError> {
        Ok(self.0.lock().unwrap().user.as_ref().filter(|u| u.email().as_str().eq_ignore_ascii_case(email.as_str())).map(copy_user))
    }
    async fn save_user(&self, _: &User) -> Result<(), RepositoryError> { unimplemented!() }
    async fn find_by_id(&self, _: &UserId) -> Result<Option<User>, RepositoryError> { unimplemented!() }
    async fn find_by_name(&self, _: &UserName) -> Result<Option<User>, RepositoryError> { unimplemented!() }
    async fn search_by_name_prefix(&self, _: &str, _: i64, _: &UserId) -> Result<Vec<(UserId, UserName)>, RepositoryError> { unimplemented!() }
    async fn get_password_hash_by_email(&self, _: &Email) -> Result<PasswordHash, RepositoryError> { unimplemented!() }
    async fn get_username_by_id(&self, _: &UserId) -> Result<UserName, RepositoryError> { unimplemented!() }
    async fn get_user_id_by_email(&self, _: &Email) -> Result<UserId, RepositoryError> { unimplemented!() }
    async fn list_oauth_connections(&self, _: &UserId) -> Result<Vec<(String, String, Option<String>)>, RepositoryError> { unimplemented!() }
    async fn link_connection(&self, _: &str, _: &str, _: &UserId, _: &str) -> Result<bool, RepositoryError> { unimplemented!() }
    async fn unlink_connection(&self, _: &UserId, _: &str) -> Result<bool, RepositoryError> { unimplemented!() }
    async fn create_connection_state(&self, _: uuid::Uuid, _: &UserId, _: &str) -> Result<(), RepositoryError> { unimplemented!() }
    async fn consume_connection_state(&self, _: uuid::Uuid) -> Result<Option<(UserId, String, DateTime<Utc>)>, RepositoryError> { unimplemented!() }
    async fn get_appearance(&self, _: &UserId) -> Result<Option<AppearanceSettings>, RepositoryError> { unimplemented!() }
    async fn get_voice_settings(&self, _: &UserId) -> Result<Option<VoiceSettings>, RepositoryError> { unimplemented!() }
    async fn get_notification_settings(&self, _: &UserId) -> Result<Option<NotificationSettings>, RepositoryError> { unimplemented!() }
    async fn get_hotkey_settings(&self, _: &UserId) -> Result<Option<HotkeyBindings>, RepositoryError> { unimplemented!() }
}

struct PlainHasher;

impl Hasher for PlainHasher {
    fn hash(&self, value: &str) -> Result<String, HashError> {
        Ok(format!("h:{value}"))
    }
    fn verify(&self, value: &str, hash: &str) -> Result<bool, HashError> {
        Ok(hash == format!("h:{value}"))
    }
}

#[async_trait]
impl AccessRevoker for Fake {
    async fn revoke_access_issued_before(&self, _user_id: &UserId, cutoff: DateTime<Utc>) -> Result<(), AccessRevokerError> {
        self.0.lock().unwrap().access_revoked_before = Some(cutoff);
        Ok(())
    }
}

fn usecases(fake: &Fake) -> (RequestPasswordReset<Fake, Fake, PlainHasher>, ResetPassword<Fake, PlainHasher, Fake>) {
    let fake = Arc::new(fake.clone());
    let hasher = Arc::new(PlainHasher);
    (
        RequestPasswordReset::new(fake.clone(), fake.clone(), hasher.clone()),
        ResetPassword::new(fake.clone(), hasher, fake),
    )
}

fn forgot(email: &str) -> RequestPasswordResetRequest {
    RequestPasswordResetRequest { email: email.to_string(), request_id: None }
}

fn reset(token: &str, password: &str) -> ResetPasswordRequest {
    ResetPasswordRequest { token: token.to_string(), new_password: password.to_string(), request_id: None }
}

#[tokio::test]
async fn a_reset_link_changes_the_password_once_and_signs_out_everywhere() {
    let (fake, _) = Fake::with_user();
    let (request, apply) = usecases(&fake);
    request.execute(forgot(" T@Example.com ")).await.unwrap();
    let token = fake.tokens().pop().unwrap();
    apply.execute(reset(&token, "new-pass")).await.unwrap();
    assert_eq!(fake.password(), "h:new-pass");
    let (refresh_revoked, access_revoked) = {
        let state = fake.0.lock().unwrap();
        (state.refresh_revoked_at.is_some(), state.access_revoked_before.is_some())
    };
    assert!(refresh_revoked && access_revoked);
    assert!(matches!(apply.execute(reset(&token, "other")).await, Err(ResetPasswordError::InvalidToken)));
    assert_eq!(fake.password(), "h:new-pass");
}

#[tokio::test]
async fn a_new_request_retires_the_previous_link() {
    let (fake, _) = Fake::with_user();
    let (request, apply) = usecases(&fake);
    request.execute(forgot("t@example.com")).await.unwrap();
    let first = fake.tokens().pop().unwrap();
    fake.age_codes(Duration::minutes(2));
    request.execute(forgot("t@example.com")).await.unwrap();
    let second = fake.tokens().pop().unwrap();
    assert_ne!(first, second);
    assert!(matches!(apply.execute(reset(&first, "new-pass")).await, Err(ResetPasswordError::InvalidToken)));
    apply.execute(reset(&second, "new-pass")).await.unwrap();
    assert!(matches!(apply.execute(reset(&first, "again")).await, Err(ResetPasswordError::InvalidToken)));
    assert_eq!(fake.password(), "h:new-pass");
}

#[tokio::test]
async fn an_expired_link_is_rejected() {
    let (fake, _) = Fake::with_user();
    let (request, apply) = usecases(&fake);
    request.execute(forgot("t@example.com")).await.unwrap();
    let token = fake.tokens().pop().unwrap();
    fake.age_codes(Duration::minutes(31));
    assert!(matches!(apply.execute(reset(&token, "new-pass")).await, Err(ResetPasswordError::Expired)));
    assert_eq!(fake.password(), "h:old");
}

#[tokio::test]
async fn unknown_emails_and_quick_repeats_look_the_same_and_send_nothing() {
    let (fake, _) = Fake::with_user();
    let (request, _) = usecases(&fake);
    request.execute(forgot("nobody@example.com")).await.unwrap();
    assert!(fake.tokens().is_empty());
    request.execute(forgot("t@example.com")).await.unwrap();
    request.execute(forgot("t@example.com")).await.unwrap();
    assert_eq!(fake.tokens().len(), 1);
}

#[tokio::test]
async fn guessed_links_do_not_burn_the_emailed_one() {
    let (fake, user_id) = Fake::with_user();
    let (request, apply) = usecases(&fake);
    request.execute(forgot("t@example.com")).await.unwrap();
    let token = fake.tokens().pop().unwrap();
    let guessed = format!("{}.{}", user_id.uuid().simple(), "0".repeat(64));
    for _ in 0..(VerificationCode::max_attempts() + 1) {
        assert!(matches!(apply.execute(reset(&guessed, "x-pass")).await, Err(ResetPasswordError::InvalidToken)));
    }
    assert!(matches!(apply.execute(reset("garbage", "new-pass")).await, Err(ResetPasswordError::InvalidToken)));
    apply.execute(reset(&token, "new-pass")).await.unwrap();
    assert_eq!(fake.password(), "h:new-pass");
}

#[tokio::test]
async fn a_wrong_secret_for_the_emailed_link_locks_it() {
    let (fake, _) = Fake::with_user();
    let (request, apply) = usecases(&fake);
    request.execute(forgot("t@example.com")).await.unwrap();
    let token = fake.tokens().pop().unwrap();
    let (id, _) = token.split_once('.').unwrap();
    let wrong = format!("{id}.{}", "0".repeat(64));
    for _ in 0..VerificationCode::max_attempts() {
        assert!(matches!(apply.execute(reset(&wrong, "x-pass")).await, Err(ResetPasswordError::InvalidToken)));
    }
    assert!(matches!(apply.execute(reset(&token, "new-pass")).await, Err(ResetPasswordError::InvalidToken)));
    assert_eq!(fake.password(), "h:old");
}

#[tokio::test]
async fn accounts_stored_with_capitals_can_reset() {
    let (fake, _) = Fake::with_email("Tester@Example.com");
    let (request, apply) = usecases(&fake);
    request.execute(forgot(" Tester@Example.com ")).await.unwrap();
    let token = fake.tokens().pop().unwrap();
    apply.execute(reset(&token, "new-pass")).await.unwrap();
    assert_eq!(fake.password(), "h:new-pass");
}

#[tokio::test]
async fn emails_typed_in_another_case_still_find_the_account() {
    let (fake, _) = Fake::with_email("Tester@Example.com");
    let (request, apply) = usecases(&fake);
    request.execute(forgot("tester@example.com")).await.unwrap();
    let token = fake.tokens().pop().unwrap();
    apply.execute(reset(&token, "new-pass")).await.unwrap();
    assert_eq!(fake.password(), "h:new-pass");
}

struct FixedTokens;

#[async_trait]
impl crate::ports::token_issuer::TokenIssuer for FixedTokens {
    async fn issue(&self, _: &UserId, _: bool) -> Result<crate::ports::token_issuer::IssuedTokens, crate::ports::token_issuer::TokenIssuerError> {
        Ok(crate::ports::token_issuer::IssuedTokens {
            access_token: "a".into(),
            refresh_token: "r".into(),
            refresh_token_id: RefreshTokenId::generate(),
            refresh_token_expires_at: Utc::now() + Duration::days(1),
            access_token_expires_in: 60,
        })
    }
    fn decode_refresh(&self, _: &str) -> Result<crate::ports::token_issuer::DecodedRefreshToken, crate::ports::token_issuer::TokenIssuerError> { unimplemented!() }
}

#[tokio::test]
async fn the_new_password_works_with_the_email_in_any_case() {
    let (fake, _) = Fake::with_email("Tester@Example.com");
    let (request, apply) = usecases(&fake);
    request.execute(forgot("tester@example.com")).await.unwrap();
    let token = fake.tokens().pop().unwrap();
    apply.execute(reset(&token, "new-pass")).await.unwrap();
    let fake = Arc::new(fake.clone());
    let login = crate::usecases::auth::login::LoginUser::new(fake.clone(), Arc::new(PlainHasher), fake, Arc::new(FixedTokens));
    let outcome = login
        .execute(crate::dto::requests::auth::login_user::LoginUserRequest {
            email: " tester@example.com ".into(),
            password: "new-pass".into(),
            client: Default::default(),
            ctx: Default::default(),
        })
        .await;
    assert!(outcome.is_ok());
}

#[tokio::test]
async fn a_password_change_after_the_email_retires_the_link() {
    let (fake, _) = Fake::with_user();
    let (request, apply) = usecases(&fake);
    request.execute(forgot("t@example.com")).await.unwrap();
    let token = fake.tokens().pop().unwrap();
    {
        let mut state = fake.0.lock().unwrap();
        let user = state.user.as_mut().unwrap();
        user.change_password(PasswordHash::new("h:changed".into()), Utc::now());
    }
    assert!(matches!(apply.execute(reset(&token, "new-pass")).await, Err(ResetPasswordError::InvalidToken)));
    assert_eq!(fake.password(), "h:changed");
}
