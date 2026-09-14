use std::sync::Arc;

use voice::ports::event_consumer::{EventConsumer, StartPosition, SubscribeOptions};
use domain::user::id::UserId;
use ports::user_profile::UserProfileRepository;
use futures::StreamExt;
use tracing::Instrument;
use infrastructure_common::messaging::nats::consumer::NatsEventConsumer;
use contracts::events::friendship::UserBlockedV1;
use contracts::events::user::{UserLifecycleV1, UserNameV1 as UserNameDto};
use voice::usecases::call::enforce_block::EnforceBlock;
use voice::usecases::call::enforce_user_deactivation::EnforceUserDeactivation;
use voice_domain::call::repository::CallRepository;
use voice::ports::friendship_lookup::FriendshipLookup;
use voice::ports::voice_service::VoiceService;

pub fn spawn<P, C, V, F>(
    jetstream: async_nats::jetstream::Context,
    subject_prefix: String,
    stream: String,
    profiles: Arc<P>,
    deactivation: Arc<EnforceUserDeactivation<C, V>>,
    block: Arc<EnforceBlock<C, V, F>>,
) where
    P: UserProfileRepository + Send + Sync + 'static,
    C: CallRepository + Send + Sync + 'static,
    V: VoiceService + Send + Sync + 'static,
    F: FriendshipLookup + Send + Sync + 'static,
{
    actix_web::rt::spawn(async move {
        let prefix = subject_prefix.trim_end_matches('.').to_string();
        let consumer = NatsEventConsumer::new(jetstream, stream.clone());
        loop {
            let opts = SubscribeOptions {
                consumer_name: "voice-projections".to_string(),
                subject: format!("{prefix}.user.>"),
                start_position: StartPosition::All,
                max_deliver: Some(5),
                dlq_subject: Some(format!("{prefix}.dlq.voice.projections")),
                extra_subjects: vec![],
                ack_wait: None,
            };
            match consumer.subscribe(opts).await {
                Ok(mut messages) => {
                    tracing::info!(%stream, "Voice projection consumer subscribed");
                    while let Some(msg) = messages.next().await {
                        match msg {
                            Ok(m) => {
                                let span = tracing::info_span!(
                                    "event",
                                    request_id = ?m.request_id,
                                    subject = %m.subject,
                                );
                                async {
                                    match handle(&m.subject, &m.payload, &*profiles, &deactivation, &block, m.request_id.clone()).await {
                                        Ok(()) => {
                                            if let Err(e) = m.ack().await {
                                                tracing::warn!(error = %e, "Voice projection ack failed");
                                            }
                                        }
                                        Err(e) => {
                                            tracing::warn!(error = %e, "Voice projection failed; message will be redelivered");
                                        }
                                    }
                                }
                                .instrument(span)
                                .await;
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, "Voice projection stream error");
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Voice projection subscribe failed; retrying");
                }
            }
            actix_web::rt::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });
}

async fn handle<P, C, V, F>(
    subject: &str,
    payload: &[u8],
    profiles: &P,
    deactivation: &EnforceUserDeactivation<C, V>,
    block: &EnforceBlock<C, V, F>,
    request_id: Option<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    P: UserProfileRepository + Send + Sync,
    C: CallRepository + Send + Sync,
    V: VoiceService + Send + Sync,
    F: FriendshipLookup + Send + Sync,
{
    if subject.ends_with(".user.blocked") {
        match serde_json::from_slice::<UserBlockedV1>(payload) {
            Ok(evt) => {
                block
                    .execute(UserId::new(evt.blocker_id), UserId::new(evt.blocked_id), request_id)
                    .await?;
            }
            Err(e) => {
                tracing::warn!(error = %e, %subject, "Malformed user block event; dropping");
            }
        }
        return Ok(());
    }
    if subject.ends_with(".user.registered") || subject.ends_with(".user.renamed") {
        match serde_json::from_slice::<UserNameDto>(payload) {
            Ok(evt) => {
                if subject.ends_with(".user.registered") {
                    profiles
                        .set_registered_at(&UserId::new(evt.user_id), evt.occurred_at)
                        .await?;
                }
                profiles
                    .upsert_name(&UserId::new(evt.user_id), &evt.name, evt.occurred_at)
                    .await?;
            }
            Err(e) => {
                tracing::warn!(error = %e, %subject, "Malformed user event; dropping");
            }
        }
        return Ok(());
    }
    if subject.ends_with(".user.profile_changed") {
        match serde_json::from_slice::<contracts::events::user::UserProfileChangedV1>(payload) {
            Ok(evt) => {
                profiles
                    .update_profile(
                        &UserId::new(evt.user_id),
                        &evt.bio,
                        evt.show_activity,
                        evt.active,
                        &evt.timezone,
                        evt.occurred_at,
                    )
                    .await?;
            }
            Err(e) => {
                tracing::warn!(error = %e, %subject, "Malformed user profile event; dropping");
            }
        }
        return Ok(());
    }
    let active = if subject.ends_with(".user.disabled") || subject.ends_with(".user.deleted") {
        Some(false)
    } else if subject.ends_with(".user.reactivated") {
        Some(true)
    } else {
        None
    };
    if let Some(active) = active {
        match serde_json::from_slice::<UserLifecycleV1>(payload) {
            Ok(evt) => {
                let applied = profiles
                    .set_active(&UserId::new(evt.user_id), active, evt.occurred_at)
                    .await?;
                if subject.ends_with(".user.deleted") {
                    profiles
                        .update_profile(&UserId::new(evt.user_id), "", false, false, "", evt.occurred_at)
                        .await?;
                }
                if !active && applied {
                    deactivation.execute(UserId::new(evt.user_id), request_id).await?;
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, %subject, "Malformed user lifecycle event; dropping");
            }
        }
    }
    Ok(())
}
