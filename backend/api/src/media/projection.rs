use std::sync::Arc;

use application::ports::{
    event_consumer::{EventConsumer, StartPosition, SubscribeOptions},
    file_storage::{FileStorage, FileStorageError},
};
use domain::{
    media::repository::MediaRepository, server::id::ServerId, user::id::UserId,
};
use futures::StreamExt;
use infrastructure::messaging::nats::consumer::NatsEventConsumer;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct ServerOwnerDto {
    server_id: Uuid,
    owner_id: Uuid,
}

#[derive(Debug, Deserialize)]
struct ServerIdDto {
    server_id: Uuid,
}

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
                consumer_name: "media-projection-server".to_string(),
                subject: format!("{prefix}.server.>"),
                start_position: StartPosition::All,
                max_deliver: Some(5),
                dlq_subject: Some(format!("{prefix}.dlq.media.server")),
            };
            match consumer.subscribe(opts).await {
                Ok(mut messages) => {
                    tracing::info!(%stream, "Media projection consumer subscribed");
                    while let Some(msg) = messages.next().await {
                        match msg {
                            Ok(m) => {
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
    if subject.ends_with(".server.created") {
        let evt: ServerOwnerDto = serde_json::from_slice(payload)?;
        media_repository
            .upsert_server_owner(&ServerId::new(evt.server_id), &UserId::new(evt.owner_id))
            .await?;
        return Ok(());
    }
    if subject.ends_with(".server.deleted") {
        let evt: ServerIdDto = serde_json::from_slice(payload)?;
        let server_id = ServerId::new(evt.server_id);
        let logo_key = media_repository.get_server_logo_key(&server_id).await?;
        media_repository.remove_server_media(&server_id).await?;
        if let Some(key) = logo_key {
            match storage.delete(&key).await {
                Ok(()) | Err(FileStorageError::NotFound) => {}
                Err(e) => {
                    tracing::warn!(error = %e, %key, "Failed to delete server logo object; cleanup queue will retry");
                }
            }
        }
        return Ok(());
    }
    Ok(())
}
