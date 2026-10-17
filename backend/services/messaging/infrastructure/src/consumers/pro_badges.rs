
use contracts::events::user::{UserEntitlementsChangedV1, UserProfileChangedV1};
use domain::user::id::UserId;
use infrastructure_common::messaging::nats::runner::{ConsumerSpec, Event, Lifecycle, spawn_consumer, subject_prefix as normalize_prefix};
use messaging_domain::badge::{BadgeEmojiStatus, BadgeRepository};
use std::sync::Arc;

pub const LEGACY_CONSUMER: &str = "messaging-pro-badges-v3";

pub fn drain_legacy<B>(jetstream: async_nats::jetstream::Context, subject_prefix: String, user_stream: String, badges: Arc<B>)
where
    B: BadgeRepository + 'static,
{
    let prefix = normalize_prefix(&subject_prefix);
    let subjects = vec![
        format!("{prefix}.user.*.entitlements_changed"),
        format!("{prefix}.user.entitlements_changed"),
        format!("{prefix}.user.*.profile_changed"),
        format!("{prefix}.user.profile_changed"),
    ];
    let spec = ConsumerSpec::new(&prefix, user_stream, LEGACY_CONSUMER, subjects);
    spawn_consumer(jetstream, spec, Lifecycle::default(), move |event: Event| {
        let badges = badges.clone();
        async move { apply(&*badges, &event.subject, &event.payload).await }
    });
}

pub async fn apply<B: BadgeRepository + ?Sized>(
    badges: &B,
    subject: &str,
    payload: &[u8],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if subject.ends_with(".entitlements_changed") {
        let Ok(evt) = serde_json::from_slice::<UserEntitlementsChangedV1>(payload) else {
            tracing::warn!(%subject, "Malformed entitlements event; dropping");
            return Ok(());
        };
        if let Some(member_number) = evt.member_number {
            badges.apply_member_number(&UserId::new(evt.user_id), member_number).await?;
        }
        return Ok(());
    }
    if subject.ends_with(".profile_changed") {
        let Ok(evt) = serde_json::from_slice::<UserProfileChangedV1>(payload) else {
            tracing::warn!(%subject, "Malformed profile event; dropping");
            return Ok(());
        };
        let emoji_status = evt.emoji_status_included.then(|| {
            evt.emoji_status_sticker_id.map(|sticker_id| BadgeEmojiStatus {
                sticker_id,
                until: evt.emoji_status_until,
            })
        });
        badges
            .apply_profile_perks(&UserId::new(evt.user_id), &evt.badge_style, emoji_status, evt.occurred_at)
            .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use messaging_domain::badge::UserBadge;
    use ports::repository::RepositoryError;
    use uuid::Uuid;

    use super::*;

    #[derive(Default)]
    struct Perks(Mutex<Vec<Option<Option<BadgeEmojiStatus>>>>);

    #[async_trait]
    impl BadgeRepository for Perks {
        async fn apply_member_number(&self, _: &UserId, _: i64) -> Result<(), RepositoryError> {
            Ok(())
        }
        async fn apply_profile_perks(&self, _: &UserId, _: &str, emoji_status: Option<Option<BadgeEmojiStatus>>, _: DateTime<Utc>) -> Result<(), RepositoryError> {
            self.0.lock().unwrap().push(emoji_status);
            Ok(())
        }
        async fn paid_badges(&self, _: &[UserId], _: DateTime<Utc>) -> Result<Vec<UserBadge>, RepositoryError> {
            Ok(Vec::new())
        }
    }

    fn event(extra: serde_json::Value) -> Vec<u8> {
        let mut base = serde_json::json!({
            "user_id": Uuid::nil(),
            "bio": "",
            "show_activity": true,
            "badge_style": "ink",
            "occurred_at": Utc::now(),
        });
        for (k, v) in extra.as_object().unwrap() {
            base[k] = v.clone();
        }
        serde_json::to_vec(&base).unwrap()
    }

    #[tokio::test]
    async fn events_from_older_identities_keep_the_status_and_new_ones_set_or_clear_it() {
        let perks = Perks::default();
        let sticker = Uuid::new_v4();
        apply(&perks, "rift.user.profile_changed", &event(serde_json::json!({}))).await.unwrap();
        apply(
            &perks,
            "rift.user.profile_changed",
            &event(serde_json::json!({ "emoji_status_included": true, "emoji_status_sticker_id": sticker })),
        )
        .await
        .unwrap();
        apply(&perks, "rift.user.profile_changed", &event(serde_json::json!({ "emoji_status_included": true })))
            .await
            .unwrap();
        let seen = perks.0.lock().unwrap().clone();
        assert_eq!(
            seen,
            vec![None, Some(Some(BadgeEmojiStatus { sticker_id: sticker, until: None })), Some(None)]
        );
    }
}
