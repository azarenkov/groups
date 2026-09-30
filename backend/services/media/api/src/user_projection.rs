use std::sync::Arc;

use media::usecases::user::update_avatar::still_avatar_key;
use contracts::events::user::{UserEntitlementsChangedV1, UserLifecycleV1 as UserLifecycleDto};
use domain::user::id::UserId;
use futures::StreamExt;
use infrastructure_common::messaging::nats::consumer::NatsEventConsumer;
use media::ports::{
    event_consumer::{EventConsumer, StartPosition, SubscribeOptions},
    file_storage::{FileStorage, FileStorageError},
};
use media_domain::banner::BannerRepository;
use media_domain::media::repository::MediaRepository;
use tracing::Instrument;

pub fn spawn<M, B, FS>(
    jetstream: async_nats::jetstream::Context,
    subject_prefix: String,
    stream: String,
    media_repository: Arc<M>,
    banners: Arc<B>,
    storage: Arc<FS>,
) where
    M: MediaRepository + Send + Sync + 'static,
    B: BannerRepository + 'static,
    FS: FileStorage + Send + Sync + 'static,
{
    actix_web::rt::spawn(async move {
        let prefix = subject_prefix.trim_end_matches('.').to_string();
        let consumer = NatsEventConsumer::new(jetstream, stream.clone());
        loop {
            let opts = SubscribeOptions {
                consumer_name: "media-projection-user".to_string(),
                subject: format!("{prefix}.user.>"),
                start_position: StartPosition::All,
                max_deliver: Some(5),
                dlq_subject: Some(format!("{prefix}.dlq.media.user")),
                extra_subjects: vec![],
                ack_wait: None,
            };
            match consumer.subscribe(opts).await {
                Ok(mut messages) => {
                    tracing::info!(%stream, "Media user projection consumer subscribed");
                    while let Some(msg) = messages.next().await {
                        match msg {
                            Ok(m) => {
                                if !m.subject.ends_with(".user.deleted") {
                                    let _ = m.ack().await;
                                    continue;
                                }
                                let span = tracing::info_span!(
                                    "event",
                                    request_id = ?m.request_id,
                                    subject = %m.subject,
                                );
                                async {
                                    match handle_deleted(&m.payload, &*media_repository, &*banners, &*storage).await {
                                        Ok(()) => {
                                            if let Err(e) = m.ack().await {
                                                tracing::warn!(error = %e, "Media user projection ack failed");
                                            }
                                        }
                                        Err(e) => {
                                            tracing::warn!(error = %e, "Media user projection failed; message will be redelivered");
                                        }
                                    }
                                }
                                .instrument(span)
                                .await;
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, "Media user projection stream error");
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Media user projection subscribe failed; retrying");
                }
            }
            actix_web::rt::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });
}

pub fn spawn_tiers<B>(
    jetstream: async_nats::jetstream::Context,
    subject_prefix: String,
    stream: String,
    banners: Arc<B>,
) where
    B: BannerRepository + 'static,
{
    actix_web::rt::spawn(async move {
        let prefix = subject_prefix.trim_end_matches('.').to_string();
        let consumer = NatsEventConsumer::new(jetstream, stream.clone());
        loop {
            let opts = SubscribeOptions {
                consumer_name: "media-projection-tiers".to_string(),
                subject: format!("{prefix}.user.*.entitlements_changed"),
                start_position: StartPosition::All,
                max_deliver: Some(20),
                dlq_subject: Some(format!("{prefix}.dlq.media.tiers")),
                extra_subjects: vec![format!("{prefix}.user.entitlements_changed")],
                ack_wait: None,
            };
            match consumer.subscribe(opts).await {
                Ok(mut messages) => {
                    tracing::info!(%stream, "Media tier projection consumer subscribed");
                    while let Some(msg) = messages.next().await {
                        match msg {
                            Ok(m) => match apply_tier(&m.payload, &*banners).await {
                                Ok(()) => {
                                    if let Err(e) = m.ack().await {
                                        tracing::warn!(error = %e, "Media tier projection ack failed");
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!(error = %e, "Tier projection failed; message will be redelivered");
                                }
                            },
                            Err(e) => {
                                tracing::warn!(error = %e, "Media tier projection stream error");
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Media tier projection subscribe failed; retrying");
                }
            }
            actix_web::rt::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });
}

async fn apply_tier<B: BannerRepository + ?Sized>(
    payload: &[u8],
    banners: &B,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let evt: UserEntitlementsChangedV1 = match serde_json::from_slice(payload) {
        Ok(evt) => evt,
        Err(e) => {
            tracing::warn!(error = %e, "Malformed entitlements event; dropping");
            return Ok(());
        }
    };
    banners
        .apply_tier(&UserId::new(evt.user_id), &evt.tier, evt.valid_until, evt.occurred_at)
        .await?;
    Ok(())
}

async fn handle_deleted<M, B, FS>(
    payload: &[u8],
    media_repository: &M,
    banners: &B,
    storage: &FS,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    M: MediaRepository + Send + Sync,
    B: BannerRepository + ?Sized,
    FS: FileStorage + Send + Sync,
{
    let evt: UserLifecycleDto = match serde_json::from_slice(payload) {
        Ok(evt) => evt,
        Err(e) => {
            tracing::warn!(error = %e, "Malformed user lifecycle event; dropping");
            return Ok(());
        }
    };
    let user_id = UserId::new(evt.user_id);
    if let Some(key) = banners.set_banner_key(&user_id, None).await? {
        match storage.delete(&key).await {
            Ok(()) | Err(FileStorageError::NotFound) => {}
            Err(e) => {
                tracing::warn!(error = %e, %key, "Failed to delete banner object of deleted user; cleanup queue will retry");
            }
        }
    }
    if let Some(key) = media_repository.mark_user_deleted(&user_id, evt.occurred_at).await? {
        match storage.delete(&key).await {
            Ok(()) | Err(FileStorageError::NotFound) => {}
            Err(e) => {
                tracing::warn!(error = %e, %key, "Failed to delete avatar object of deleted user; cleanup queue will retry");
            }
        }
        if let Some(still_key) = still_avatar_key(&key)
            && let Err(e) = storage.delete(&still_key).await
            && !matches!(e, FileStorageError::NotFound)
        {
            tracing::warn!(error = %e, key = %still_key, "Failed to delete the still avatar frame of deleted user");
        }
    }
    Ok(())
}
