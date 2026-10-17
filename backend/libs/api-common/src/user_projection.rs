use std::{future::Future, sync::Arc};

use contracts::events::user::{UserLifecycleV1 as UserLifecycleDto, UserNameV1 as UserNameDto, UserProfileChangedV1 as UserProfileChangedDto};
use domain::user::id::UserId;
use infrastructure_common::messaging::nats::runner::{ConsumerSpec, Event, HandlerError, Lifecycle, spawn_consumer, subject_prefix};
use ports::user_profile::UserProfileRepository;

pub fn spawn<P>(
    jetstream: async_nats::jetstream::Context,
    subject_prefix: String,
    stream: String,
    consumer_name: &'static str,
    repository: Arc<P>,
) where
    P: UserProfileRepository + Send + Sync + 'static,
{
    spawn_with(jetstream, subject_prefix, stream, consumer_name, repository, |_, _| async { Ok(()) });
}

pub fn spawn_with<P, X, Fut>(
    jetstream: async_nats::jetstream::Context,
    prefix: String,
    stream: String,
    consumer_name: &'static str,
    repository: Arc<P>,
    extension: X,
) where
    P: UserProfileRepository + Send + Sync + 'static,
    X: Fn(String, Vec<u8>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), HandlerError>> + Send + 'static,
{
    let prefix = subject_prefix(&prefix);
    let spec = ConsumerSpec::new(&prefix, stream, consumer_name, vec![format!("{prefix}.user.>")]);
    let extension = Arc::new(extension);
    spawn_consumer(jetstream, spec, Lifecycle::default(), move |event: Event| {
        let repository = repository.clone();
        let extension = extension.clone();
        async move {
            apply(repository.as_ref(), &event.subject, &event.payload).await?;
            extension(event.subject, event.payload).await
        }
    });
}

pub async fn apply<P>(repository: &P, subject: &str, payload: &[u8]) -> Result<(), HandlerError>
where
    P: UserProfileRepository + Send + Sync + ?Sized,
{
    if subject.ends_with(".user.registered") || subject.ends_with(".user.renamed") {
        let Ok(evt) = serde_json::from_slice::<UserNameDto>(payload) else {
            tracing::warn!(%subject, "Malformed user event; dropping");
            return Ok(());
        };
        let user_id = UserId::new(evt.user_id);
        if subject.ends_with(".user.registered") {
            repository.set_registered_at(&user_id, evt.occurred_at).await?;
        }
        repository.upsert_name(&user_id, &evt.name, evt.occurred_at).await?;
        return Ok(());
    }
    if subject.ends_with(".user.profile_changed") {
        let Ok(evt) = serde_json::from_slice::<UserProfileChangedDto>(payload) else {
            tracing::warn!(%subject, "Malformed user profile event; dropping");
            return Ok(());
        };
        repository
            .update_profile(&UserId::new(evt.user_id), &evt.bio, evt.show_activity, evt.active, &evt.timezone, evt.occurred_at)
            .await?;
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
        let Ok(evt) = serde_json::from_slice::<UserLifecycleDto>(payload) else {
            tracing::warn!(%subject, "Malformed user lifecycle event; dropping");
            return Ok(());
        };
        let user_id = UserId::new(evt.user_id);
        repository.set_active(&user_id, active, evt.occurred_at).await?;
        if subject.ends_with(".user.deleted") {
            repository.update_profile(&user_id, "", false, false, "", evt.occurred_at).await?;
        }
    }
    Ok(())
}
