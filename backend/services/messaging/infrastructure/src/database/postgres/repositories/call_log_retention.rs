use async_trait::async_trait;
use chrono::{DateTime, Utc};
use messaging::ports::call_log_retention::CallLogRetention;
use ports::repository::RepositoryError;
use sqlx::{Pool, Postgres};

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

pub struct PostgresCallLogRetention {
    pool: Pool<Postgres>,
}

impl PostgresCallLogRetention {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CallLogRetention for PostgresCallLogRetention {
    async fn forget_consumed_before(&self, before: DateTime<Utc>, limit: i64) -> Result<u64, RepositoryError> {
        let result = sqlx::query(
            r#"
            DELETE FROM call_log_consumed
            WHERE ctid IN (
                SELECT ctid FROM call_log_consumed
                WHERE created_at < $1
                ORDER BY created_at
                LIMIT $2
            )
            "#,
        )
        .bind(before)
        .bind(limit)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(result.rows_affected())
    }
}
