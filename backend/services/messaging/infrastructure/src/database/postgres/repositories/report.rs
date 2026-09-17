use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use messaging_domain::report::{Report, ReportRepository, ReportTarget};
use ports::repository::RepositoryError;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

pub struct PostgresReportRepository {
    pool: Pool<Postgres>,
}

impl PostgresReportRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReportRepository for PostgresReportRepository {
    async fn insert(&self, report: &Report) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            INSERT INTO reports (
                id, reporter_id, target_type, target_id, target_name, reason, details,
                context_channel_id, context_message_id, status, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
            report.id(),
            report.reporter_id().uuid(),
            report.target().as_str(),
            report.target_id(),
            report.target_name(),
            report.reason().as_str(),
            report.details(),
            report.context().channel_id,
            report.context().message_id,
            report.status().as_str(),
            report.created_at(),
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn has_recent_report(
        &self,
        reporter_id: &UserId,
        target: ReportTarget,
        target_id: Uuid,
        since: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM reports
                WHERE reporter_id = $1
                  AND target_type = $2
                  AND target_id = $3
                  AND created_at >= $4
            ) AS "exists!"
            "#,
            reporter_id.uuid(),
            target.as_str(),
            target_id,
            since,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.exists)
    }

    async fn count_recent_by_reporter(
        &self,
        reporter_id: &UserId,
        since: DateTime<Utc>,
    ) -> Result<i64, RepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT COUNT(*) AS "count!"
            FROM reports
            WHERE reporter_id = $1 AND created_at >= $2
            "#,
            reporter_id.uuid(),
            since,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.count)
    }
}
