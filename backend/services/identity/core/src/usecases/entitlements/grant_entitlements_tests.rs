use std::sync::Arc;

use chrono::{Duration, Utc};
use identity_domain::user::{emoji_status::EmojiStatus, tier::Tier};
use uuid::Uuid;

use super::*;
use crate::usecases::entitlements::test_fake::Fake;

#[tokio::test]
async fn renewing_after_a_lapse_drops_the_status_from_the_old_plan() {
    let (fake, id) = Fake::with_tier(Tier::Pro);
    {
        let mut state = fake.0.lock().unwrap();
        state.entitlements.valid_until = Some(Utc::now() - Duration::minutes(5));
        let user = state.user.as_mut().unwrap();
        user.set_emoji_status(Some(EmojiStatus::new(Uuid::new_v4(), None)), Utc::now()).unwrap();
        user.pull_events();
    }
    GrantEntitlements::new(Arc::new(fake.clone()))
        .execute(id, "pro", Some(Utc::now() + Duration::days(30)), "manual")
        .await
        .unwrap();
    assert_eq!(fake.status(), None);
    assert!(fake.events().iter().any(|e| e.contains("ProfileChanged")));
}

#[tokio::test]
async fn renewing_an_active_plan_keeps_the_status() {
    let (fake, id) = Fake::with_tier(Tier::Pro);
    let status = EmojiStatus::new(Uuid::new_v4(), None);
    {
        let mut state = fake.0.lock().unwrap();
        state.entitlements.valid_until = Some(Utc::now() + Duration::days(1));
        state.user.as_mut().unwrap().set_emoji_status(Some(status), Utc::now()).unwrap();
        state.user.as_mut().unwrap().pull_events();
    }
    GrantEntitlements::new(Arc::new(fake.clone()))
        .execute(id, "pro", Some(Utc::now() + Duration::days(30)), "manual")
        .await
        .unwrap();
    assert_eq!(fake.status(), Some(status));
}
