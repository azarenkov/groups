use std::sync::Arc;

use media::ports::{
    event_consumer::{EventConsumer, StartPosition, SubscribeOptions},
    file_storage::{FileStorage, FileStorageError},
};
use media_domain::media::repository::MediaRepository;
use domain::{community::id::CommunityId, user::id::UserId};
use futures::StreamExt;
use tracing::Instrument;
use infrastructure_common::messaging::nats::consumer::NatsEventConsumer;
use contracts::events::community::{
    CommunityCreatedV1 as CommunityOwnerDto, CommunityDeletedV1 as CommunityDeletedDto,
    CommunityOwnerChangedV1 as CommunityOwnerChangedDto,
};

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
    actix_web::rt::spawn(async move {
        let prefix = subject_prefix.trim_end_matches('.').to_string();
        let consumer = NatsEventConsumer::new(jetstream, stream.clone());
        loop {
            let opts = SubscribeOptions {
                consumer_name: "media-projection-community".to_string(),
                subject: format!("{prefix}.community.>"),
                start_position: StartPosition::All,
                max_deliver: Some(5),
                dlq_subject: Some(format!("{prefix}.dlq.media.community")),
                extra_subjects: vec![],
                ack_wait: None,
            };
            match consumer.subscribe(opts).await {
                Ok(mut messages) => {
                    tracing::info!(%stream, "Media projection consumer subscribed");
                    while let Some(msg) = messages.next().await {
                        match msg {
                            Ok(m) => {
                                let span = tracing::info_span!(
                                    "event",
                                    request_id = ?m.request_id,
                                    subject = %m.subject,
                                );
                                async {
                                    match handle(&m.subject, &m.payload, &*media_repository, &*storage)
                                        .await
                                    {
                                        Ok(()) => {
                                            if let Err(e) = m.ack().await {
                                                tracing::warn!(error = %e, "Media projection ack failed");
                                            }
                                        }
                                        Err(e) => {
                                            tracing::warn!(error = %e, "Media projection handler failed; message will be redelivered");
                                        }
                                    }
                                }
                                .instrument(span)
                                .await;
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, "Media projection stream error");
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Media projection subscribe failed; retrying");
                }
            }
            actix_web::rt::time::sleep(std::time::Duration::from_secs(5)).await;
        }
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
