use std::sync::Arc;

use domain::user::id::UserId;
use ports::user_profile::UserProfileRepository;
use contracts::events::friendship::UserBlockedV1;
use contracts::events::user::{UserLifecycleV1, UserNameV1 as UserNameDto};
use voice::usecases::call::enforce_block::EnforceBlock;
use voice::usecases::call::enforce_user_deactivation::EnforceUserDeactivation;
use voice_domain::call::repository::CallRepository;
use voice::ports::friendship_lookup::FriendshipLookup;
use voice::ports::voice_service::VoiceService;
use infrastructure_common::messaging::nats::runner::{ConsumerSpec, Event, Lifecycle, spawn_consumer, subject_prefix as normalize_prefix};

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
    let prefix = normalize_prefix(&subject_prefix);
    let spec = ConsumerSpec::new(&prefix, stream, "voice-projections", vec![format!("{prefix}.user.>")]);
    spawn_consumer(jetstream, spec, Lifecycle::default(), move |event: Event| {
        let (profiles, deactivation, block) = (profiles.clone(), deactivation.clone(), block.clone());
        async move { handle(&event.subject, &event.payload, &*profiles, &deactivation, &block, event.request_id).await }
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
                    .set_active(&UserId::new(evt.user_id), evt.active, evt.occurred_at)
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
