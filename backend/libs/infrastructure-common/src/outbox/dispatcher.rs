use std::sync::Arc;

use ports::{outbox::record::AggregateType, outbox::record::OutboxRecord, outbox::record::PartitionSequence, outbox_publisher::OutboxPublisher, repository::RepositoryError};
use chrono::{Duration, Utc};
use sqlx::{Pool, Postgres};

use crate::database::postgres::{
    dto::aggregate_type::AggregateTypeDto, error::map_sqlx_error_to_domain_error,
};

#[derive(Debug, thiserror::Error)]
pub enum OutboxDispatchError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct OutboxBacklog {
    pub pending: i64,
    pub stuck: i64,
    pub oldest_pending_secs: f64,
}

pub struct OutboxDispatcher<P: OutboxPublisher> {
    source: String,
    pool: Pool<Postgres>,
    publisher: Arc<P>,
    max_attempts: i16,
    backoff_base_secs: i32,
    backoff_max_multiplier: i32,
}

impl<P: OutboxPublisher> OutboxDispatcher<P> {
    pub fn new(
        source: String,
        pool: Pool<Postgres>,
        publisher: Arc<P>,
        max_attempts: i16,
        backoff_base_secs: i32,
        backoff_max_multiplier: i32,
    ) -> Self {
        Self {
            source,
            pool,
            publisher,
            max_attempts,
            backoff_base_secs,
            backoff_max_multiplier,
        }
    }

    pub async fn process_one(&self) -> Result<bool, OutboxDispatchError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let row = sqlx::query!(
            r#"
            SELECT
                id,
                aggregate_id,
                aggregate_type AS "aggregate_type: AggregateTypeDto",
                parent_id,
                parent_type AS "parent_type: AggregateTypeDto",
                event_type,
                payload,
                occurred_at,
                request_id,
                attempts,
                partition_key,
                partition_seq
            FROM outbox o
            WHERE processed_at IS NULL
              AND (
                  last_attempt_at IS NULL
                  OR last_attempt_at < now()
                     - (LEAST(attempts::int, $2) * $1 * interval '1 second')
              )
              AND NOT EXISTS (
                  SELECT 1 FROM outbox earlier
                  WHERE earlier.partition_key = o.partition_key
                    AND earlier.processed_at IS NULL
                    AND earlier.seq < o.seq
              )
            ORDER BY last_attempt_at NULLS FIRST, seq
            FOR UPDATE SKIP LOCKED
            LIMIT 1
            "#,
            self.backoff_base_secs,
            self.backoff_max_multiplier,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        let Some(row) = row else {
            tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
            return Ok(false);
        };

        let record = OutboxRecord {
            id: row.id,
            aggregate_id: row.aggregate_id,
            aggregate_type: AggregateType::from(row.aggregate_type),
            parent_id: row.parent_id,
            parent_type: row.parent_type.map(AggregateType::from),
            event_type: row.event_type,
            payload: row.payload,
            occurred_at: row.occurred_at,
            request_id: row.request_id,
        };

        let dispatch_span = tracing::info_span!(
            "outbox_dispatch",
            outbox_id = %record.id,
            event_type = %record.event_type,
            request_id = ?record.request_id,
        );
        let _enter = dispatch_span.enter();

        let sequence = match (row.partition_key, row.partition_seq) {
            (Some(partition), Some(seq)) => Some(PartitionSequence {
                partition: crate::messaging::partitions::partition_id(&self.source, partition),
                seq,
                members: crate::messaging::partitions::conversation_members(&record),
            }),
            _ => None,
        };
        match self.publisher.publish(&record, sequence).await {
            Ok(()) => {
                sqlx::query!(
                    r#"
                    UPDATE outbox
                    SET processed_at = now()
                    WHERE id = $1
                    "#,
                    record.id,
                )
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
            }
            Err(e) => {
                if row.attempts + 1 >= self.max_attempts {
                    tracing::error!(
                        outbox_id = %record.id,
                        event_type = %record.event_type,
                        attempts = row.attempts + 1,
                        error = %e,
                        "Outbox publish keeps failing; later events of this partition are held back",
                    );
                } else {
                    tracing::warn!(
                        outbox_id = %record.id,
                        event_type = %record.event_type,
                        error = %e,
                        "Outbox publish failed",
                    );
                }
                sqlx::query!(
                    r#"
                    UPDATE outbox
                    SET attempts = LEAST(attempts + 1, 100),
                        last_error = $2,
                        last_attempt_at = now()
                    WHERE id = $1
                    "#,
                    record.id,
                    e.to_string(),
                )
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
            }
        }

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;

        Ok(true)
    }

    pub async fn backlog(&self) -> Result<OutboxBacklog, OutboxDispatchError> {
        let row = sqlx::query!(
            r#"
            SELECT
                count(*) AS "pending!",
                count(*) FILTER (WHERE attempts >= $1) AS "stuck!",
                COALESCE(EXTRACT(EPOCH FROM now() - min(created_at)), 0)::float8 AS "oldest_pending_secs!"
            FROM outbox
            WHERE processed_at IS NULL
            "#,
            self.max_attempts,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(OutboxBacklog {
            pending: row.pending,
            stuck: row.stuck,
            oldest_pending_secs: row.oldest_pending_secs,
        })
    }

    pub async fn cleanup_processed(
        &self,
        retention_secs: u64,
        batch_size: i64,
    ) -> Result<u64, OutboxDispatchError> {
        let cutoff = Utc::now() - Duration::seconds(retention_secs as i64);

        let result = sqlx::query!(
            r#"
            DELETE FROM outbox
            WHERE id IN (
                SELECT id FROM outbox
                WHERE processed_at IS NOT NULL
                  AND processed_at < $1
                ORDER BY processed_at
                LIMIT $2
            )
            "#,
            cutoff,
            batch_size,
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(result.rows_affected())
    }
}
