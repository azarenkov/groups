use chrono::Utc;
use contracts::events::storage::{STORAGE_KEYS_RELEASED, StorageKeysReleasedV1};
use ports::{
    outbox::record::{AggregateType, OutboxRecord},
    repository::RepositoryError,
};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::database::postgres::error::map_sqlx_error_to_domain_error;
use crate::database::postgres::repositories::outbox_record::save_outbox_records;

const MAX_KEYS_PER_RELEASE: usize = 500;

pub(crate) async fn lock_storage_keys(
    tx: &mut Transaction<'_, Postgres>,
    storage_keys: &[String],
) -> Result<(), RepositoryError> {
    let mut keys = storage_keys.to_vec();
    keys.sort();
    keys.dedup();
    for key in &keys {
        sqlx::query!("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))", key)
            .execute(&mut **tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
    }
    Ok(())
}

pub(crate) async fn ensure_storage_keys_linked(
    tx: &mut Transaction<'_, Postgres>,
    storage_keys: &[String],
) -> Result<(), RepositoryError> {
    if storage_keys.is_empty() {
        return Ok(());
    }
    lock_storage_keys(tx, storage_keys).await?;
    let missing = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) AS "count!"
        FROM unnest($1::text[]) AS key
        WHERE NOT EXISTS (SELECT 1 FROM message_attachments a WHERE a.storage_key = key)
          AND NOT EXISTS (SELECT 1 FROM dm_message_attachments a WHERE a.storage_key = key)
        "#,
        storage_keys,
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?;
    if missing > 0 {
        return Err(RepositoryError::NotFound);
    }
    Ok(())
}

pub(crate) async fn release_storage_keys(
    tx: &mut Transaction<'_, Postgres>,
    storage_keys: &[String],
) -> Result<(), RepositoryError> {
    if storage_keys.is_empty() {
        return Ok(());
    }
    lock_storage_keys(tx, storage_keys).await?;
    let releasable = sqlx::query_scalar!(
        r#"
        SELECT key AS "key!"
        FROM unnest($1::text[]) AS key
        WHERE NOT EXISTS (
            SELECT 1 FROM reports WHERE context_attachment_keys @> ARRAY[key]
        )
          AND NOT EXISTS (
            SELECT 1 FROM message_attachments a WHERE a.storage_key = key
        )
          AND NOT EXISTS (
            SELECT 1 FROM dm_message_attachments a WHERE a.storage_key = key
        )
        "#,
        storage_keys,
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?;
    let records = storage_release_records(&releasable);
    if records.is_empty() {
        return Ok(());
    }
    save_outbox_records(tx, &records).await
}

pub struct PostgresStorageReleases {
    pool: sqlx::Pool<Postgres>,
}

impl PostgresStorageReleases {
    pub fn new(pool: sqlx::Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl messaging::ports::storage_releases::StorageReleases for PostgresStorageReleases {
    async fn release(&self, storage_keys: &[String]) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error_to_domain_error)?;
        release_storage_keys(&mut tx, storage_keys).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)
    }
}

fn storage_release_records(storage_keys: &[String]) -> Vec<OutboxRecord> {
    let mut keys = storage_keys.to_vec();
    keys.sort();
    keys.dedup();
    let occurred_at = Utc::now();
    keys.chunks(MAX_KEYS_PER_RELEASE)
        .map(|chunk| {
            let payload = StorageKeysReleasedV1 {
                storage_keys: chunk.to_vec(),
                occurred_at,
            };
            OutboxRecord {
                id: Uuid::new_v4(),
                aggregate_id: Uuid::new_v4(),
                aggregate_type: AggregateType::Storage,
                parent_id: None,
                parent_type: None,
                event_type: STORAGE_KEYS_RELEASED.to_string(),
                payload: serde_json::to_value(&payload).unwrap_or_default(),
                occurred_at,
                request_id: None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{MAX_KEYS_PER_RELEASE, storage_release_records};
    use contracts::events::storage::StorageKeysReleasedV1;
    use ports::outbox::record::AggregateType;

    fn keys_of(record: &ports::outbox::record::OutboxRecord) -> Vec<String> {
        serde_json::from_value::<StorageKeysReleasedV1>(record.payload.clone())
            .unwrap()
            .storage_keys
    }

    #[test]
    fn no_record_without_keys() {
        assert!(storage_release_records(&[]).is_empty());
    }

    #[test]
    fn record_carries_unique_sorted_keys() {
        let keys = vec!["b".to_string(), "a".to_string(), "b".to_string()];
        let records = storage_release_records(&keys);
        assert_eq!(records.len(), 1);
        assert!(matches!(records[0].aggregate_type, AggregateType::Storage));
        assert_eq!(records[0].event_type, "storage.keys_released");
        assert_eq!(keys_of(&records[0]), vec!["a", "b"]);
    }

    #[test]
    fn large_releases_are_split_into_bounded_records() {
        let keys: Vec<String> = (0..MAX_KEYS_PER_RELEASE * 2 + 1)
            .map(|i| format!("attachments/{i:06}"))
            .collect();
        let records = storage_release_records(&keys);
        let sizes: Vec<usize> = records.iter().map(|r| keys_of(r).len()).collect();
        assert_eq!(sizes, vec![MAX_KEYS_PER_RELEASE, MAX_KEYS_PER_RELEASE, 1]);
        let mut all: Vec<String> = records.iter().flat_map(keys_of).collect();
        all.sort();
        assert_eq!(all, keys);
        let ids: std::collections::HashSet<_> = records.iter().map(|r| r.id).collect();
        assert_eq!(ids.len(), records.len());
    }
}
