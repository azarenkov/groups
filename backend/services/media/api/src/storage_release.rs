use std::sync::Arc;

use contracts::events::storage::{STORAGE_KEYS_RELEASED, StorageKeysReleasedV1};
use media_domain::attachment::repository::AttachmentRepository;
use infrastructure_common::messaging::nats::runner::{ConsumerSpec, Event, Lifecycle, spawn_consumer, subject_prefix as normalize_prefix};

pub fn spawn<A>(
    jetstream: async_nats::jetstream::Context,
    subject_prefix: String,
    stream: String,
    attachment_repository: Arc<A>,
) where
    A: AttachmentRepository + Send + Sync + 'static,
{
    let prefix = normalize_prefix(&subject_prefix);
    let spec = ConsumerSpec::new(&prefix, stream, "media-storage-release", vec![format!("{prefix}.storage.>")]);
    let released_suffix = format!(".{STORAGE_KEYS_RELEASED}");
    spawn_consumer(jetstream, spec, Lifecycle::default(), move |event: Event| {
        let attachment_repository = attachment_repository.clone();
        let released = event.subject.ends_with(&released_suffix);
        async move {
            if !released {
                return Ok(());
            }
            handle_released(&event.payload, &*attachment_repository).await
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
