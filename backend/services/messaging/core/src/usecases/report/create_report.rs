use std::sync::Arc;

use chrono::{Duration, Utc};
use domain::user::id::UserId;
use messaging_domain::report::{
    DAILY_REPORT_LIMIT, DUPLICATE_WINDOW_HOURS, Report, ReportContext, ReportError, ReportReason,
    ReportRepository, ReportTarget,
};
use domain::server::id::ServerId;
use messaging_domain::server::repository::ServerRepository;
use ports::{repository::RepositoryError, user_profile::UserProfileRepository};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum CreateReportError {
    #[error("Unknown report reason")]
    UnknownReason,
    #[error("Reported target not found")]
    TargetNotFound,
    #[error("Report already sent")]
    AlreadyReported,
    #[error("Too many reports")]
    TooManyReports,
    #[error("Report error: {0}")]
    Domain(#[from] ReportError),
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct CreateReportRequest {
    pub target: ReportTarget,
    pub target_id: Uuid,
    pub reason: String,
    pub details: Option<String>,
    pub context: ReportContext,
}

pub struct CreateReport<R, U, S>
where
    R: ReportRepository,
    U: UserProfileRepository,
    S: ServerRepository,
{
    report_repository: Arc<R>,
    user_repository: Arc<U>,
    server_repository: Arc<S>,
}

impl<R, U, S> CreateReport<R, U, S>
where
    R: ReportRepository,
    U: UserProfileRepository,
    S: ServerRepository,
{
    pub fn new(
        report_repository: Arc<R>,
        user_repository: Arc<U>,
        server_repository: Arc<S>,
    ) -> Self {
        Self {
            report_repository,
            user_repository,
            server_repository,
        }
    }

    pub async fn execute(
        &self,
        reporter_id: UserId,
        request: CreateReportRequest,
    ) -> Result<(), CreateReportError> {
        let reason =
            ReportReason::parse(&request.reason).ok_or(CreateReportError::UnknownReason)?;
        let target_name = self.target_name(request.target, request.target_id).await?;

        let now = Utc::now();
        let report = Report::create(
            reporter_id,
            request.target,
            request.target_id,
            target_name,
            reason,
            request.details,
            request.context,
            now,
        )?;

        let window_start = now - Duration::hours(DUPLICATE_WINDOW_HOURS);
        if self
            .report_repository
            .has_recent_report(&reporter_id, request.target, request.target_id, window_start)
            .await?
        {
            return Err(CreateReportError::AlreadyReported);
        }
        if self
            .report_repository
            .count_recent_by_reporter(&reporter_id, window_start)
            .await?
            >= DAILY_REPORT_LIMIT
        {
            return Err(CreateReportError::TooManyReports);
        }

        self.report_repository.insert(&report).await?;
        Ok(())
    }

    async fn target_name(
        &self,
        target: ReportTarget,
        target_id: Uuid,
    ) -> Result<String, CreateReportError> {
        match target {
            ReportTarget::User => self
                .user_repository
                .get_name(&UserId::new(target_id))
                .await?
                .ok_or(CreateReportError::TargetNotFound),
            ReportTarget::Server => self
                .server_repository
                .find_server_by_id(&ServerId::new(target_id))
                .await?
                .map(|server| server.name().as_str().to_string())
                .ok_or(CreateReportError::TargetNotFound),
        }
    }
}
