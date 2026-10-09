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

use domain::user::id::UserId;
use identity_domain::user::emoji_status::EmojiStatus;
use ports::repository::RepositoryError;
use uuid::Uuid;

use crate::ports::sticker_lookup::{StickerLookup, StickerLookupError};

#[derive(Default)]
pub(super) struct State {
    pub(super) user: Option<User>,
    pub(super) entitlements: Entitlements,
    pub(super) events: Vec<String>,
    pub(super) unusable_sticker: bool,
}

#[derive(Clone, Default)]
pub(super) struct Fake(pub(super) Arc<Mutex<State>>);

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
    pub(super) fn with_tier(tier: Tier) -> (Self, UserId) {
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
            state.entitlements.tier = tier;
        }
        (fake, id)
    }

    pub(super) fn status(&self) -> Option<EmojiStatus> {
        self.0.lock().unwrap().user.as_ref().and_then(|u| u.emoji_status())
    }

    pub(super) fn events(&self) -> Vec<String> {
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
        let state = self.0.lock().unwrap();
        Ok(Entitlements { tier: state.entitlements.tier, valid_until: state.entitlements.valid_until, ..Entitlements::default() })
    }
    async fn save_entitlements(&mut self, _: &UserId, tier: &str, valid_until: Option<DateTime<Utc>>, _: &str) -> Result<Option<i64>, RepositoryError> {
        let mut state = self.0.lock().unwrap();
        state.entitlements.tier = Tier::parse(tier).unwrap_or_default();
        state.entitlements.valid_until = valid_until;
        Ok(None)
    }
    async fn expired_entitlement_users(&mut self, now: DateTime<Utc>, _: i64) -> Result<Vec<UserId>, RepositoryError> {
        let state = self.0.lock().unwrap();
        let lapsed = state.entitlements.tier != Tier::Free && state.entitlements.valid_until.is_some_and(|until| until <= now);
        Ok(state.user.as_ref().filter(|_| lapsed).map(|u| vec![*u.id()]).unwrap_or_default())
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
    async fn trial_history(&mut self, _: &Email, _: Option<&str>, _: DateTime<Utc>) -> Result<TrialHistory, RepositoryError> { unimplemented!() }
    async fn start_trial(&mut self, _: &UserId, _: &Email, _: Option<&str>, _: DateTime<Utc>, _: DateTime<Utc>) -> Result<(), RepositoryError> { unimplemented!() }
    async fn load_trials_ending_for_update(&mut self, _: DateTime<Utc>, _: i64) -> Result<Vec<UserId>, RepositoryError> { unimplemented!() }
    async fn mark_trial_reminded(&mut self, _: &UserId, _: DateTime<Utc>) -> Result<(), RepositoryError> { unimplemented!() }
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
