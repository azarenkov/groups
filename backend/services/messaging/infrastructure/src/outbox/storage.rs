use chrono::Utc;
use contracts::events::storage::{STORAGE_KEYS_RELEASED, StorageKeysReleasedV1};
use ports::{
    outbox::record::{AggregateType, OutboxRecord},
    repository::RepositoryError,
};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::database::postgres::repositories::outbox_record::save_outbox_records;

pub(crate) async fn release_storage_keys(
    tx: &mut Transaction<'_, Postgres>,
    storage_keys: &[String],
) -> Result<(), RepositoryError> {
    let Some(record) = storage_release_record(storage_keys) else {
        return Ok(());
    };
    save_outbox_records(tx, std::slice::from_ref(&record)).await
}

fn storage_release_record(storage_keys: &[String]) -> Option<OutboxRecord> {
    if storage_keys.is_empty() {
        return None;
    }
    let mut keys = storage_keys.to_vec();
    keys.sort();
    keys.dedup();
    let payload = StorageKeysReleasedV1 {
        storage_keys: keys,
        occurred_at: Utc::now(),
    };
    Some(OutboxRecord {
        id: Uuid::new_v4(),
        aggregate_id: Uuid::new_v4(),
        aggregate_type: AggregateType::Storage,
        parent_id: None,
        parent_type: None,
        event_type: STORAGE_KEYS_RELEASED.to_string(),
        payload: serde_json::to_value(&payload).unwrap_or_default(),
        occurred_at: payload.occurred_at,
        request_id: None,
    })
}

#[cfg(test)]
mod tests {
    use super::storage_release_record;
    use contracts::events::storage::StorageKeysReleasedV1;
    use ports::outbox::record::AggregateType;

    #[test]
    fn no_record_without_keys() {
        assert!(storage_release_record(&[]).is_none());
    }

    #[test]
    fn record_carries_unique_sorted_keys() {
        let keys = vec!["b".to_string(), "a".to_string(), "b".to_string()];
        let record = storage_release_record(&keys).unwrap();
        assert!(matches!(record.aggregate_type, AggregateType::Storage));
        assert_eq!(record.event_type, "storage.keys_released");
        let payload: StorageKeysReleasedV1 = serde_json::from_value(record.payload).unwrap();
        assert_eq!(payload.storage_keys, vec!["a", "b"]);
    }
}
