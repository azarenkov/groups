use async_trait::async_trait;
use chrono::{DateTime, Utc};
use messaging_domain::report::{Report, ReportOutcome, ReportRepository};
use ports::repository::RepositoryError;
use sqlx::{Pool, Postgres};

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
    async fn insert_within_quota(
        &self,
        report: &Report,
        since: DateTime<Utc>,
        daily_limit: i64,
    ) -> Result<ReportOutcome, RepositoryError> {
        let reporter_id = report.reporter_id().uuid();
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        sqlx::query!(
            r#"SELECT pg_advisory_xact_lock(hashtext($1::text))"#,
            reporter_id.to_string(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        let quota = sqlx::query!(
            r#"
            SELECT
                COUNT(*) AS "sent!",
                COUNT(*) FILTER (
                    WHERE target_type = $2 AND target_id = $3
                ) AS "for_target!"
            FROM reports
            WHERE reporter_id = $1 AND created_at >= $4
            "#,
            reporter_id,
            report.target().as_str(),
            report.target_id(),
            since,
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if quota.for_target > 0 {
            return Ok(ReportOutcome::AlreadyReported);
        }
        if quota.sent >= daily_limit {
            return Ok(ReportOutcome::LimitReached);
        }

        let context = report.context();
        sqlx::query!(
            r#"
            INSERT INTO reports (
                id, reporter_id, target_type, target_id, target_name, reason, details,
                context_channel_id, context_message_id, context_message_content, status, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
            report.id(),
            reporter_id,
            report.target().as_str(),
            report.target_id(),
            report.target_name(),
            report.reason().as_str(),
            report.details(),
            context.channel_id,
            context.message_id,
            context.message_content,
            report.status().as_str(),
            report.created_at(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(ReportOutcome::Stored)
    }
}
