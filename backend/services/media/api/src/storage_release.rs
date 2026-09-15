use std::sync::Arc;

use contracts::events::storage::{STORAGE_KEYS_RELEASED, StorageKeysReleasedV1};
use futures::StreamExt;
use infrastructure_common::messaging::nats::consumer::NatsEventConsumer;
use media::ports::event_consumer::{EventConsumer, StartPosition, SubscribeOptions};
use media_domain::attachment::repository::AttachmentRepository;
use tracing::Instrument;

pub fn spawn<A>(
    jetstream: async_nats::jetstream::Context,
    subject_prefix: String,
    stream: String,
    attachment_repository: Arc<A>,
) where
    A: AttachmentRepository + Send + Sync + 'static,
{
    actix_web::rt::spawn(async move {
        let prefix = subject_prefix.trim_end_matches('.').to_string();
        let consumer = NatsEventConsumer::new(jetstream, stream.clone());
        let released_suffix = format!(".{STORAGE_KEYS_RELEASED}");
        loop {
            let opts = SubscribeOptions {
                consumer_name: "media-storage-release".to_string(),
                subject: format!("{prefix}.storage.>"),
                start_position: StartPosition::All,
                max_deliver: Some(5),
                dlq_subject: Some(format!("{prefix}.dlq.media.storage")),
                extra_subjects: vec![],
                ack_wait: None,
            };
            match consumer.subscribe(opts).await {
                Ok(mut messages) => {
                    tracing::info!(%stream, "Media storage release consumer subscribed");
                    while let Some(msg) = messages.next().await {
                        match msg {
                            Ok(m) => {
                                if !m.subject.ends_with(&released_suffix) {
                                    let _ = m.ack().await;
                                    continue;
                                }
                                let span = tracing::info_span!(
                                    "event",
                                    request_id = ?m.request_id,
                                    subject = %m.subject,
                                );
                                async {
                                    match handle_released(&m.payload, &*attachment_repository).await {
                                        Ok(()) => {
                                            if let Err(e) = m.ack().await {
                                                tracing::warn!(error = %e, "Media storage release ack failed");
                                            }
                                        }
                                        Err(e) => {
                                            tracing::warn!(error = %e, "Media storage release failed; message will be redelivered");
                                        }
                                    }
                                }
                                .instrument(span)
                                .await;
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, "Media storage release stream error");
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Media storage release subscribe failed; retrying");
                }
            }
            actix_web::rt::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });
}

async fn handle_released<A>(
    payload: &[u8],
    attachment_repository: &A,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    A: AttachmentRepository + Send + Sync,
{
    let Some(keys) = released_keys(payload) else {
        return Ok(());
    };
    attachment_repository.enqueue_cleanup(&keys).await?;
    Ok(())
}

fn released_keys(payload: &[u8]) -> Option<Vec<String>> {
    match serde_json::from_slice::<StorageKeysReleasedV1>(payload) {
        Ok(evt) => {
            let keys: Vec<String> = evt.storage_keys.into_iter().filter(|k| !k.is_empty()).collect();
            (!keys.is_empty()).then_some(keys)
        }
        Err(e) => {
            tracing::warn!(error = %e, "Malformed storage release event; dropping");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::released_keys;

    #[test]
    fn extracts_non_empty_keys() {
        let payload = br#"{"storage_keys":["a/1","","b/2"],"occurred_at":"2026-09-16T00:00:00Z"}"#;
        assert_eq!(released_keys(payload), Some(vec!["a/1".to_string(), "b/2".to_string()]));
    }

    #[test]
    fn ignores_empty_and_malformed_events() {
        assert_eq!(released_keys(br#"{"storage_keys":[],"occurred_at":"2026-09-16T00:00:00Z"}"#), None);
        assert_eq!(released_keys(b"not json"), None);
    }
}
