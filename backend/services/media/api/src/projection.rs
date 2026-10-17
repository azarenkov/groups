use std::sync::Arc;

use media::ports::file_storage::{FileStorage, FileStorageError};
use media_domain::media::repository::MediaRepository;
use domain::{community::id::CommunityId, user::id::UserId};
use contracts::events::community::{
    CommunityCreatedV1 as CommunityOwnerDto, CommunityDeletedV1 as CommunityDeletedDto,
    CommunityOwnerChangedV1 as CommunityOwnerChangedDto,
};
use infrastructure_common::messaging::nats::runner::{ConsumerSpec, Event, Lifecycle, spawn_consumer, subject_prefix as normalize_prefix};

pub fn spawn<M, FS>(
    jetstream: async_nats::jetstream::Context,
    subject_prefix: String,
    stream: String,
    media_repository: Arc<M>,
    storage: Arc<FS>,
) where
    M: MediaRepository + Send + Sync + 'static,
    FS: FileStorage + Send + Sync + 'static,
{
    let prefix = normalize_prefix(&subject_prefix);
    let spec = ConsumerSpec::new(&prefix, stream, "media-projection-community", vec![format!("{prefix}.community.>")]);
    spawn_consumer(jetstream, spec, Lifecycle::default(), move |event: Event| {
        let (media_repository, storage) = (media_repository.clone(), storage.clone());
        async move { handle(&event.subject, &event.payload, &*media_repository, &*storage).await }
    });
}

async fn handle<M, FS>(
    subject: &str,
    payload: &[u8],
    media_repository: &M,
    storage: &FS,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    M: MediaRepository + Send + Sync,
    FS: FileStorage + Send + Sync,
{
    if subject.ends_with(".community.created") {
        let evt: CommunityOwnerDto = serde_json::from_slice(payload)?;
        media_repository
            .upsert_community_owner(
                &CommunityId::new(evt.community_id),
                &UserId::new(evt.owner_id),
                evt.occurred_at,
            )
            .await?;
        return Ok(());
    }
    if subject.ends_with(".community.owner_changed") {
        let evt: CommunityOwnerChangedDto = serde_json::from_slice(payload)?;
        media_repository
            .upsert_community_owner(
                &CommunityId::new(evt.community_id),
                &UserId::new(evt.owner_id),
                evt.occurred_at,
            )
            .await?;
        return Ok(());
    }
    if subject.ends_with(".community.deleted") {
        let evt: CommunityDeletedDto = serde_json::from_slice(payload)?;
        let community_id = CommunityId::new(evt.community_id);
        let logo_key = media_repository.get_community_logo_key(&community_id).await?;
        media_repository
            .remove_community_media(&community_id, &UserId::new(evt.owner_id), evt.occurred_at)
            .await?;
        if let Some(key) = logo_key {
            match storage.delete(&key).await {
                Ok(()) | Err(FileStorageError::NotFound) => {}
                Err(e) => {
                    tracing::warn!(error = %e, %key, "Failed to delete community logo object; cleanup queue will retry");
                }
            }
        }
        return Ok(());
    }
    Ok(())
}
