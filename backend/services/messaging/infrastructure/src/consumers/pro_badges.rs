use std::sync::Arc;

use contracts::events::user::{UserEntitlementsChangedV1, UserProfileChangedV1};
use domain::user::id::UserId;
use futures::StreamExt;
use infrastructure_common::messaging::nats::consumer::NatsEventConsumer;
use messaging::ports::event_consumer::{EventConsumer, StartPosition, SubscribeOptions};
use messaging_domain::badge::{BadgeEmojiStatus, BadgeRepository};

const MAX_DELIVER: u32 = 20;
const RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(5);

pub fn spawn<B>(
    jetstream: async_nats::jetstream::Context,
    subject_prefix: String,
    user_stream: String,
    badges: Arc<B>,
) where
    B: BadgeRepository + 'static,
{
    let prefix = subject_prefix.trim_end_matches('.').to_string();
    tokio::spawn(async move {
        let consumer = NatsEventConsumer::new(jetstream, user_stream.clone());
        loop {
            let opts = SubscribeOptions {
                consumer_name: "messaging-pro-badges-v3".to_string(),
                subject: format!("{prefix}.user.*.entitlements_changed"),
                start_position: StartPosition::All,
                max_deliver: Some(MAX_DELIVER),
                dlq_subject: Some(format!("{prefix}.dlq.messaging.pro_badges")),
                extra_subjects: vec![
                    format!("{prefix}.user.entitlements_changed"),
                    format!("{prefix}.user.*.profile_changed"),
                    format!("{prefix}.user.profile_changed"),
                ],
                ack_wait: None,
            };
            match consumer.subscribe(opts).await {
                Ok(mut messages) => {
                    tracing::info!(stream = %user_stream, "Pro badge projection subscribed");
                    while let Some(msg) = messages.next().await {
                        let Ok(m) = msg else {
                            tracing::warn!("Pro badge stream error");
                            break;
                        };
                        match apply(&*badges, &m.subject, &m.payload).await {
                            Ok(()) => {
                                let _ = m.ack().await;
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, subject = %m.subject, "Pro badge projection failed; scheduling retry");
                                let _ = m.retry_after(RETRY_DELAY).await;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Pro badge subscribe failed; retrying");
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
    });
}

async fn apply<B: BadgeRepository + ?Sized>(
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
