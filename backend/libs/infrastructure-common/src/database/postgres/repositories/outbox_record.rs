use ports::{outbox::record::OutboxRecord, repository::RepositoryError};
use sqlx::{Pool, Postgres, Transaction};
use uuid::Uuid;

use crate::messaging::partitions::{is_conversation_partition, outbox_partition_key};
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
        let partition_key = outbox_partition_key(record);
        let partition_seq = if is_conversation_partition(record) {
            Some(
                sqlx::query_scalar!(
                    r#"
                    INSERT INTO outbox_partition_seq (partition_key, seq)
                    VALUES ($1, 1)
                    ON CONFLICT (partition_key) DO UPDATE SET seq = outbox_partition_seq.seq + 1
                    RETURNING seq
                    "#,
                    partition_key,
                )
                .fetch_one(&mut **tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?,
            )
        } else {
            None
        };
        let aggregate_type = AggregateTypeDto::from(&record.aggregate_type);
        let parent_type = record.parent_type.as_ref().map(AggregateTypeDto::from);

        sqlx::query!(
            r#"
            INSERT INTO outbox (
                id, aggregate_id, aggregate_type,
                parent_id, parent_type,
                event_type, payload, occurred_at, request_id, partition_key, partition_seq
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
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
            partition_key,
            partition_seq,
        )
        .execute(&mut **tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
    }
    Ok(())
}

#[derive(Clone)]
pub struct PostgresPartitionSequences {
    pool: Pool<Postgres>,
}

impl PostgresPartitionSequences {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    pub async fn current(&self, partition: Uuid) -> Result<i64, RepositoryError> {
        let seq = sqlx::query_scalar!(
            r#"SELECT seq FROM outbox_partition_seq WHERE partition_key = $1"#,
            partition,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(seq.unwrap_or(0))
    }
}
