use std::sync::Arc;

use application::ports::outbox_publisher::OutboxPublisher;
use chrono::{Duration, Utc};
use domain::shared::{
    outbox::outbox_record::{AggregateType, OutboxRecord},
    repository::error::RepositoryError,
};
use sqlx::{Pool, Postgres};

use crate::database::postgres::{
    dto::aggregate_type::AggregateTypeDto, error::map_sqlx_error_to_domain_error,
};

#[derive(Debug, thiserror::Error)]
pub enum OutboxDispatchError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct OutboxDispatcher<P: OutboxPublisher> {
    pool: Pool<Postgres>,
    publisher: Arc<P>,
}

impl<P: OutboxPublisher> OutboxDispatcher<P> {
    pub fn new(pool: Pool<Postgres>, publisher: Arc<P>) -> Self {
        Self { pool, publisher }
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
                occurred_at
            FROM outbox
            WHERE processed_at IS NULL
            ORDER BY created_at
            FOR UPDATE SKIP LOCKED
            LIMIT 1
            "#,
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
        };

        match self.publisher.publish(&record).await {
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
                tracing::warn!(
                    outbox_id = %record.id,
                    event_type = %record.event_type,
                    error = %e,
                    "Outbox publish failed",
                );
                sqlx::query!(
                    r#"
                    UPDATE outbox
                    SET attempts = attempts + 1,
                        last_error = $2
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
