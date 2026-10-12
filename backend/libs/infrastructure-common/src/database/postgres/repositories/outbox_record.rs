use ports::{outbox::record::OutboxRecord, repository::RepositoryError};
use sqlx::{Postgres, Transaction};

use crate::messaging::partitions::outbox_partition_key;
use crate::database::postgres::{
    dto::aggregate_type::AggregateTypeDto, error::map_sqlx_error_to_domain_error,
};
pub async fn save_outbox_records(
    tx: &mut Transaction<'_, Postgres>,
    records: &[OutboxRecord],
) -> Result<(), RepositoryError> {
    let mut partitions: Vec<String> = records.iter().map(|r| outbox_partition_key(r).to_string()).collect();
    partitions.sort();
    partitions.dedup();
    for partition in &partitions {
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 60))")
            .bind(partition)
            .execute(&mut **tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
    }
    for record in records {
        let aggregate_type = AggregateTypeDto::from(&record.aggregate_type);
        let parent_type = record.parent_type.as_ref().map(AggregateTypeDto::from);

        sqlx::query!(
            r#"
            INSERT INTO outbox (
                id, aggregate_id, aggregate_type,
                parent_id, parent_type,
                event_type, payload, occurred_at, request_id, partition_key
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
            record.id,
            record.aggregate_id,
            aggregate_type as AggregateTypeDto,
            record.parent_id,
            parent_type as Option<AggregateTypeDto>,
            record.event_type,
            record.payload,
            record.occurred_at,
            record.request_id,
            outbox_partition_key(record),
        )
        .execute(&mut **tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
    }
    Ok(())
}
