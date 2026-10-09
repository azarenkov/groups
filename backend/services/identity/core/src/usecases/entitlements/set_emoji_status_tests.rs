use std::sync::Arc;

use chrono::Utc;
use identity_domain::user::tier::Tier;
use uuid::Uuid;

use super::*;
use crate::usecases::entitlements::test_fake::Fake;

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
