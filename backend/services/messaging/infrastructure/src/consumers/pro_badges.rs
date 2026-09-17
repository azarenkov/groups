use std::sync::Arc;

use contracts::events::user::{UserEntitlementsChangedV1, UserProfileChangedV1};
use domain::user::id::UserId;
use futures::StreamExt;
use infrastructure_common::messaging::nats::consumer::NatsEventConsumer;
use messaging::ports::event_consumer::{EventConsumer, StartPosition, SubscribeOptions};
use messaging_domain::badge::BadgeRepository;

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
                consumer_name: "messaging-pro-badges".to_string(),
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
        badges
            .apply_tier(&UserId::new(evt.user_id), &evt.tier, evt.valid_until, evt.occurred_at)
            .await?;
        return Ok(());
    }
    if subject.ends_with(".profile_changed") {
        let Ok(evt) = serde_json::from_slice::<UserProfileChangedV1>(payload) else {
            tracing::warn!(%subject, "Malformed profile event; dropping");
            return Ok(());
        };
        badges
            .apply_badge_style(&UserId::new(evt.user_id), &evt.badge_style, evt.occurred_at)
            .await?;
    }
    Ok(())
}
