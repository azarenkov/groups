use async_trait::async_trait;
use sqlx::{Pool, Postgres};

use crate::messaging::nats::sequence_gate::SequenceProgress;

pub struct PostgresSequenceProgress {
    pool: Pool<Postgres>,
}

impl PostgresSequenceProgress {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SequenceProgress for PostgresSequenceProgress {
    async fn load(&self, partition: &str) -> Result<Option<i64>, String> {
        sqlx::query_scalar!("SELECT seq FROM realtime_partition_progress WHERE partition = $1", partition)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())
    }

    async fn save(&self, progress: &[(String, i64)]) -> Result<(), String> {
        if progress.is_empty() {
            return Ok(());
        }
        let (partitions, seqs): (Vec<String>, Vec<i64>) = progress.iter().cloned().unzip();
        sqlx::query!(
            r#"
            INSERT INTO realtime_partition_progress (partition, seq)
            SELECT * FROM UNNEST($1::text[], $2::bigint[])
            ON CONFLICT (partition) DO UPDATE
            SET seq = GREATEST(realtime_partition_progress.seq, EXCLUDED.seq),
                updated_at = now()
            "#,
            &partitions,
            &seqs,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }
}
