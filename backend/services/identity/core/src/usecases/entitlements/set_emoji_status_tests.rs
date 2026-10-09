use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::{password_hash::PasswordHash, user::email::Email, user::locale::Locale, user::name::UserName};
use identity_domain::{
    events::IdentityEvent,
    refresh_token::{RefreshToken, id::RefreshTokenId},
    unit_of_work::{IdentityTransaction, IdentityUnitOfWork},
    user::{User, tier::{Entitlements, Tier, TrialHistory}},
    user::settings::{appearance::AppearanceSettings, hotkeys::HotkeyBindings, notification::NotificationSettings, voice::VoiceSettings},
    verification::{code::VerificationCode, id::VerificationCodeId, purpose::VerificationPurpose, throttle::IssuedCodes},
};

use super::*;

#[derive(Default)]
struct State {
    user: Option<User>,
    tier: Tier,
    events: Vec<String>,
    unusable_sticker: bool,
}

#[derive(Clone, Default)]
struct Fake(Arc<Mutex<State>>);

fn copy(user: &User) -> User {
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

impl Fake {
    fn with_tier(tier: Tier) -> (Self, UserId) {
        let mut user = User::create(
            UserName::new("tester".to_string()),
            Email::new("t@example.com".to_string()),
            PasswordHash::new("h".to_string()),
            None,
            None,
            Locale::default(),
        );
        user.verify_email(Utc::now());
        user.pull_events();
        let id = *user.id();
        let fake = Self::default();
        {
            let mut state = fake.0.lock().unwrap();
            state.user = Some(user);
            state.tier = tier;
        }
        (fake, id)
    }

    fn status(&self) -> Option<EmojiStatus> {
        self.0.lock().unwrap().user.as_ref().and_then(|u| u.emoji_status())
    }

    fn events(&self) -> Vec<String> {
        self.0.lock().unwrap().events.clone()
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
    async fn load_user_for_update(&mut self, _: &UserId) -> Result<Option<User>, RepositoryError> {
        Ok(self.0.lock().unwrap().user.as_ref().map(copy))
    }
    async fn save_user(&mut self, user: &User) -> Result<(), RepositoryError> {
        self.0.lock().unwrap().user = Some(copy(user));
        Ok(())
    }
    async fn save_events(&mut self, events: &[IdentityEvent], _: Option<&str>) -> Result<(), RepositoryError> {
        let mut state = self.0.lock().unwrap();
        state.events.extend(events.iter().map(|e| format!("{e:?}")));
        Ok(())
    }
    async fn load_entitlements(&mut self, _: &UserId) -> Result<Entitlements, RepositoryError> {
        Ok(Entitlements { tier: self.0.lock().unwrap().tier, ..Entitlements::default() })
    }
    async fn commit(self: Box<Self>) -> Result<(), RepositoryError> {
        Ok(())
    }
    async fn load_pending_verification_for_purpose_for_update(&mut self, _: &UserId, _: VerificationPurpose) -> Result<Option<VerificationCode>, RepositoryError> { unimplemented!() }
    async fn revoke_refresh_tokens_for_user(&mut self, _: &UserId, _: Option<&RefreshTokenId>, _: DateTime<Utc>) -> Result<(), RepositoryError> { unimplemented!() }
    async fn insert_verification_code(&mut self, _: &VerificationCode) -> Result<(), RepositoryError> { unimplemented!() }
    async fn save_verification_code(&mut self, _: &VerificationCode) -> Result<(), RepositoryError> { unimplemented!() }
    async fn verification_code_owner(&mut self, _: &VerificationCodeId) -> Result<Option<UserId>, RepositoryError> { unimplemented!() }
    async fn load_verification_code_for_update(&mut self, _: &VerificationCodeId) -> Result<Option<VerificationCode>, RepositoryError> { unimplemented!() }
    async fn consume_pending_verifications(&mut self, _: &UserId, _: VerificationPurpose, _: DateTime<Utc>) -> Result<(), RepositoryError> { unimplemented!() }
    async fn issued_verification_codes(&mut self, _: &UserId, _: VerificationPurpose, _: DateTime<Utc>) -> Result<IssuedCodes, RepositoryError> { unimplemented!() }
    async fn load_stale_unverified_users_for_update(&mut self, _: DateTime<Utc>, _: i64) -> Result<Vec<User>, RepositoryError> { unimplemented!() }
    async fn load_pending_verification_for_update(&mut self, _: &UserId) -> Result<Option<VerificationCode>, RepositoryError> { unimplemented!() }
    async fn delete_oauth_identities(&mut self, _: &UserId) -> Result<(), RepositoryError> { unimplemented!() }
    async fn load_refresh_token_for_update(&mut self, _: &RefreshTokenId) -> Result<Option<RefreshToken>, RepositoryError> { unimplemented!() }
    async fn insert_user(&mut self, _: &User) -> Result<(), RepositoryError> { unimplemented!() }
    async fn insert_refresh_token(&mut self, _: &RefreshToken) -> Result<(), RepositoryError> { unimplemented!() }
    async fn save_refresh_token(&mut self, _: &RefreshToken) -> Result<(), RepositoryError> { unimplemented!() }
    async fn lock_user(&mut self, _: &UserId) -> Result<bool, RepositoryError> { unimplemented!() }
    async fn save_entitlements(&mut self, _: &UserId, _: &str, _: Option<DateTime<Utc>>, _: &str) -> Result<Option<i64>, RepositoryError> { unimplemented!() }
    async fn trial_history(&mut self, _: &Email, _: Option<&str>, _: DateTime<Utc>) -> Result<TrialHistory, RepositoryError> { unimplemented!() }
    async fn start_trial(&mut self, _: &UserId, _: &Email, _: Option<&str>, _: DateTime<Utc>, _: DateTime<Utc>) -> Result<(), RepositoryError> { unimplemented!() }
    async fn load_trials_ending_for_update(&mut self, _: DateTime<Utc>, _: i64) -> Result<Vec<UserId>, RepositoryError> { unimplemented!() }
    async fn mark_trial_reminded(&mut self, _: &UserId, _: DateTime<Utc>) -> Result<(), RepositoryError> { unimplemented!() }
    async fn load_expired_entitlements_for_update(&mut self, _: DateTime<Utc>, _: i64) -> Result<Vec<UserId>, RepositoryError> { unimplemented!() }
    async fn load_appearance_for_update(&mut self, _: &UserId) -> Result<Option<AppearanceSettings>, RepositoryError> { unimplemented!() }
    async fn save_appearance(&mut self, _: &UserId, _: &AppearanceSettings) -> Result<(), RepositoryError> { unimplemented!() }
    async fn load_voice_settings_for_update(&mut self, _: &UserId) -> Result<Option<VoiceSettings>, RepositoryError> { unimplemented!() }
    async fn save_voice_settings(&mut self, _: &UserId, _: &VoiceSettings) -> Result<(), RepositoryError> { unimplemented!() }
    async fn load_notification_settings_for_update(&mut self, _: &UserId) -> Result<Option<NotificationSettings>, RepositoryError> { unimplemented!() }
    async fn save_notification_settings(&mut self, _: &UserId, _: &NotificationSettings) -> Result<(), RepositoryError> { unimplemented!() }
    async fn load_hotkey_settings_for_update(&mut self, _: &UserId) -> Result<Option<HotkeyBindings>, RepositoryError> { unimplemented!() }
    async fn save_hotkey_settings(&mut self, _: &UserId, _: &HotkeyBindings) -> Result<(), RepositoryError> { unimplemented!() }
}

#[async_trait]
impl StickerLookup for Fake {
    async fn usable_sticker(&self, _: Uuid) -> Result<bool, StickerLookupError> {
        Ok(!self.0.lock().unwrap().unusable_sticker)
    }
}

fn usecase(fake: &Fake) -> SetEmojiStatus<Fake, Fake> {
    SetEmojiStatus::new(Arc::new(fake.clone()), Arc::new(fake.clone()))
}

#[tokio::test]
async fn a_pro_user_sets_a_status_for_a_while_and_others_hear_about_it() {
    let (fake, id) = Fake::with_tier(Tier::Pro);
    let sticker = Uuid::new_v4();
    let before = Utc::now();
    let status = usecase(&fake).set(id, sticker, Some("8h"), None).await.unwrap();
    assert_eq!(fake.status(), Some(status));
    assert_eq!(status.sticker_id(), sticker);
    let until = status.until().unwrap();
    assert!(until >= before + chrono::Duration::hours(8) && until <= Utc::now() + chrono::Duration::hours(8));
    let events = fake.events();
    assert_eq!(events.len(), 1);
    assert!(events[0].contains("ProfileChanged"));
    assert!(events[0].contains(&sticker.to_string()));
}

#[tokio::test]
async fn a_status_without_a_duration_stays_until_removed() {
    let (fake, id) = Fake::with_tier(Tier::Pro);
    let status = usecase(&fake).set(id, Uuid::new_v4(), None, None).await.unwrap();
    assert_eq!(status.until(), None);
    usecase(&fake).clear(id, None).await.unwrap();
    assert_eq!(fake.status(), None);
    assert_eq!(fake.events().len(), 2);
}

#[tokio::test]
async fn free_users_cannot_set_a_status_but_can_clear_one() {
    let (fake, id) = Fake::with_tier(Tier::Free);
    let err = usecase(&fake).set(id, Uuid::new_v4(), None, None).await.unwrap_err();
    assert!(matches!(err, SetEmojiStatusError::ProRequired));
    assert_eq!(fake.status(), None);
    usecase(&fake).clear(id, None).await.unwrap();
    assert!(fake.events().is_empty());
}

#[tokio::test]
async fn unknown_stickers_and_durations_are_rejected() {
    let (fake, id) = Fake::with_tier(Tier::Pro);
    let err = usecase(&fake).set(id, Uuid::new_v4(), Some("2y"), None).await.unwrap_err();
    assert!(matches!(err, SetEmojiStatusError::UnknownDuration));
    fake.0.lock().unwrap().unusable_sticker = true;
    let err = usecase(&fake).set(id, Uuid::new_v4(), None, None).await.unwrap_err();
    assert!(matches!(err, SetEmojiStatusError::StickerNotFound));
    assert_eq!(fake.status(), None);
}
