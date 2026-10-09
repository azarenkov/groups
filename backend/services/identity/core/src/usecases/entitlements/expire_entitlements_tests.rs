use std::sync::Arc;

use chrono::{Duration, Utc};
use identity_domain::user::{emoji_status::EmojiStatus, tier::{BadgeStyle, Tier}};
use uuid::Uuid;

use super::*;
use crate::usecases::entitlements::test_fake::Fake;

fn lapsed_pro() -> Fake {
    let (fake, _) = Fake::with_tier(Tier::Pro);
    {
        let mut state = fake.0.lock().unwrap();
        state.entitlements.valid_until = Some(Utc::now() - Duration::minutes(1));
        let user = state.user.as_mut().unwrap();
        user.set_badge_style(BadgeStyle::Ink, Utc::now()).unwrap();
        user.set_emoji_status(Some(EmojiStatus::new(Uuid::new_v4(), None)), Utc::now()).unwrap();
        user.pull_events();
    }
    fake
}

#[tokio::test]
async fn a_lapsed_plan_takes_the_status_and_badge_style_with_it() {
    let fake = lapsed_pro();
    let expired = ExpireEntitlements::new(Arc::new(fake.clone()), 10).drain().await.unwrap();
    assert_eq!(expired, 1);
    let state = fake.0.lock().unwrap();
    assert_eq!(state.entitlements.tier, Tier::Free);
    let user = state.user.as_ref().unwrap();
    assert_eq!(user.emoji_status(), None);
    assert_eq!(user.badge_style(), BadgeStyle::Default);
    assert!(state.events.iter().any(|e| e.contains("EntitlementsChanged")));
    assert!(state.events.iter().any(|e| e.contains("ProfileChanged")));
}

#[tokio::test]
async fn a_plan_renewed_before_expiry_runs_is_left_alone() {
    let (fake, _) = Fake::with_tier(Tier::Pro);
    fake.0.lock().unwrap().entitlements.valid_until = Some(Utc::now() + Duration::days(30));
    let expired = ExpireEntitlements::new(Arc::new(fake.clone()), 10).drain().await.unwrap();
    assert_eq!(expired, 0);
    assert_eq!(fake.0.lock().unwrap().entitlements.tier, Tier::Pro);
    assert!(fake.events().is_empty());
}
