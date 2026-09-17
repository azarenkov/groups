use std::sync::Arc;

use chrono::{Duration, Utc};
use domain::server::id::ServerId;
use domain::user::id::UserId;
use messaging_domain::channel::id::ChannelId;
use messaging_domain::channel::repository::ChannelRepository;
use messaging_domain::dm_message::id::DmMessageId;
use messaging_domain::dm_message::repository::DmMessageRepository;
use messaging_domain::message::id::MessageId;
use messaging_domain::message::repository::MessageRepository;
use messaging_domain::report::{
    DAILY_REPORT_LIMIT, DUPLICATE_WINDOW_HOURS, Report, ReportContext, ReportError, ReportOutcome,
    ReportReason, ReportRepository, ReportTarget,
};
use messaging_domain::server::repository::ServerRepository;
use ports::{repository::RepositoryError, user_profile::UserProfileRepository};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum CreateReportError {
    #[error("Unknown report reason")]
    UnknownReason,
    #[error("Reported target not found")]
    TargetNotFound,
    #[error("Reported message does not belong to this report")]
    ContextMismatch,
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
    pub message_id: Option<Uuid>,
}

pub struct CreateReport<R, U, S, M, D, C>
where
    R: ReportRepository,
    U: UserProfileRepository,
    S: ServerRepository,
    M: MessageRepository,
    D: DmMessageRepository,
    C: ChannelRepository,
{
    report_repository: Arc<R>,
    user_repository: Arc<U>,
    server_repository: Arc<S>,
    message_repository: Arc<M>,
    dm_repository: Arc<D>,
    channel_repository: Arc<C>,
}

impl<R, U, S, M, D, C> CreateReport<R, U, S, M, D, C>
where
    R: ReportRepository,
    U: UserProfileRepository,
    S: ServerRepository,
    M: MessageRepository,
    D: DmMessageRepository,
    C: ChannelRepository,
{
    pub fn new(
        report_repository: Arc<R>,
        user_repository: Arc<U>,
        server_repository: Arc<S>,
        message_repository: Arc<M>,
        dm_repository: Arc<D>,
        channel_repository: Arc<C>,
    ) -> Self {
        Self {
            report_repository,
            user_repository,
            server_repository,
            message_repository,
            dm_repository,
            channel_repository,
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
        let context = self
            .resolve_context(&reporter_id, request.target, request.target_id, request.message_id)
            .await?;

        let now = Utc::now();
        let report = Report::create(
            reporter_id,
            request.target,
            request.target_id,
            target_name,
            reason,
            request.details,
            context,
            now,
        )?;

        let window_start = now - Duration::hours(DUPLICATE_WINDOW_HOURS);
        match self
            .report_repository
            .insert_within_quota(&report, window_start, DAILY_REPORT_LIMIT)
            .await?
        {
            ReportOutcome::Stored => Ok(()),
            ReportOutcome::AlreadyReported => Err(CreateReportError::AlreadyReported),
            ReportOutcome::LimitReached => Err(CreateReportError::TooManyReports),
        }
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

    async fn resolve_context(
        &self,
        reporter_id: &UserId,
        target: ReportTarget,
        target_id: Uuid,
        message_id: Option<Uuid>,
    ) -> Result<ReportContext, CreateReportError> {
        let Some(message_id) = message_id else {
            return Ok(ReportContext::default());
        };

        match self
            .message_repository
            .find_message_by_id(&MessageId::new(message_id))
            .await
        {
            Ok(message) => {
                let channel_id = *message.channel_id();
                let belongs = match target {
                    ReportTarget::User => message.author_id().uuid() == target_id,
                    ReportTarget::Server => {
                        self.channel_server_id(&channel_id).await? == Some(target_id)
                    }
                };
                if !belongs {
                    return Err(CreateReportError::ContextMismatch);
                }
                if !self
                    .channel_repository
                    .is_text_channel_member(&channel_id, reporter_id)
                    .await?
                {
                    return Err(CreateReportError::ContextMismatch);
                }
                return Ok(ReportContext {
                    channel_id: Some(channel_id.uuid()),
                    message_id: Some(message_id),
                    message_content: Some(message.content().as_str().to_string()),
                });
            }
            Err(RepositoryError::NotFound) => {}
            Err(e) => return Err(e.into()),
        }

        match self
            .dm_repository
            .find_by_id(&DmMessageId::new(message_id))
            .await
        {
            Ok(message) => {
                if target != ReportTarget::User
                    || message.sender_id().uuid() != target_id
                    || message.recipient_id() != reporter_id
                {
                    return Err(CreateReportError::ContextMismatch);
                }
                Ok(ReportContext {
                    channel_id: None,
                    message_id: Some(message_id),
                    message_content: Some(message.content().as_str().to_string()),
                })
            }
            Err(RepositoryError::NotFound) => Err(CreateReportError::ContextMismatch),
            Err(e) => Err(e.into()),
        }
    }

    async fn channel_server_id(
        &self,
        channel_id: &ChannelId,
    ) -> Result<Option<Uuid>, CreateReportError> {
        match self.channel_repository.find_channel_by_id(channel_id).await {
            Ok(channel) => Ok(Some(channel.server_id().uuid())),
            Err(RepositoryError::NotFound) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
