use std::sync::Arc;

use contracts::events::user::UserEntitlementsChangedV1;
use domain::{channel::id::ChannelId, stream::StreamCeiling, user::id::UserId};
use futures::StreamExt;
use infrastructure_common::messaging::nats::consumer::NatsEventConsumer;
use ports::user_profile::UserProfileRepository;
use voice::ports::event_consumer::{EventConsumer, StartPosition, SubscribeOptions};
use voice::ports::voice_service::VoiceService;
use voice_domain::call::repository::CallRepository;

pub fn spawn<P, C, V>(
    jetstream: async_nats::jetstream::Context,
    subject_prefix: String,
    stream: String,
    profiles: Arc<P>,
    calls: Arc<C>,
    voice: Arc<V>,
) where
    P: UserProfileRepository + Send + Sync + 'static,
    C: CallRepository + Send + Sync + 'static,
    V: VoiceService + Send + Sync + 'static,
{
    actix_web::rt::spawn(async move {
        let prefix = subject_prefix.trim_end_matches('.').to_string();
        let consumer = NatsEventConsumer::new(jetstream, stream.clone());
        loop {
            let opts = SubscribeOptions {
                consumer_name: "voice-projection-tiers".to_string(),
                subject: format!("{prefix}.user.*.entitlements_changed"),
                start_position: StartPosition::All,
                max_deliver: Some(20),
                dlq_subject: Some(format!("{prefix}.dlq.voice.tiers")),
                extra_subjects: vec![format!("{prefix}.user.entitlements_changed")],
                ack_wait: None,
            };
            match consumer.subscribe(opts).await {
                Ok(mut messages) => {
                    tracing::info!(%stream, "Voice tier projection subscribed");
                    while let Some(msg) = messages.next().await {
                        let Ok(m) = msg else {
                            tracing::warn!("Voice tier projection stream error");
                            break;
                        };
                        match apply(&*profiles, &*calls, &*voice, &m.payload).await {
                            Ok(()) => {
                                let _ = m.ack().await;
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, "Voice tier projection failed; scheduling retry");
                                let _ = m.retry_after(std::time::Duration::from_secs(5)).await;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Voice tier projection subscribe failed; retrying");
                }
            }
            actix_web::rt::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });
}

async fn apply<P, C, V>(
    profiles: &P,
    calls: &C,
    voice: &V,
    payload: &[u8],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    P: UserProfileRepository + Send + Sync,
    C: CallRepository + Send + Sync,
    V: VoiceService + Send + Sync,
{
    let Ok(evt) = serde_json::from_slice::<UserEntitlementsChangedV1>(payload) else {
        tracing::warn!("Malformed entitlements event; dropping");
        return Ok(());
    };
    let user = UserId::new(evt.user_id);
    profiles.apply_tier(&user, &evt.tier, evt.valid_until, evt.occurred_at).await?;
    if chrono::Utc::now() - evt.occurred_at > chrono::Duration::hours(1) {
        return Ok(());
    }
    let Some(call_id) = calls.active_call_for(&user).await? else {
        return Ok(());
    };
    let room = ChannelId::new(call_id);
    let ceiling = StreamCeiling::for_tier(&profiles.tier_of(&user, chrono::Utc::now()).await?);
    voice.set_stream_ceiling(&room, &user, ceiling).await?;
    Ok(())
}
