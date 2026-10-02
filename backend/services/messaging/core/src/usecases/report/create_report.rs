use std::sync::Arc;

use chrono::{Duration, Utc};
use domain::community::id::CommunityId;
use domain::user::id::UserId;
use messaging_domain::channel::visibility::ChannelVisibility;
use messaging_domain::channel::repository::ChannelRepository;
use messaging_domain::dm_message::id::DmMessageId;
use messaging_domain::dm_message::repository::DmMessageRepository;
use messaging_domain::message::id::MessageId;
use messaging_domain::message::repository::MessageRepository;
use messaging_domain::report::{
    DAILY_REPORT_LIMIT, DUPLICATE_WINDOW_HOURS, Report, ReportContext, ReportError, ReportOutcome,
    ReportReason, ReportRepository, ReportTarget,
};
use messaging_domain::community::repository::CommunityRepository;
use ports::{repository::RepositoryError, user_profile::UserProfileRepository};

use crate::ports::sticker_packs::StickerPackDirectory;
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
    #[error("Report target lookup unavailable: {0}")]
    Unavailable(String),
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

pub struct CreateReport<R, U, S, M, D, C, P>
where
    R: ReportRepository,
    U: UserProfileRepository,
    S: CommunityRepository,
    M: MessageRepository,
    D: DmMessageRepository,
    C: ChannelRepository,
    P: StickerPackDirectory,
{
    report_repository: Arc<R>,
    user_repository: Arc<U>,
    community_repository: Arc<S>,
    message_repository: Arc<M>,
    dm_repository: Arc<D>,
    channel_repository: Arc<C>,
    sticker_packs: Arc<P>,
}

impl<R, U, S, M, D, C, P> CreateReport<R, U, S, M, D, C, P>
where
    R: ReportRepository,
    U: UserProfileRepository,
    S: CommunityRepository,
    M: MessageRepository,
    D: DmMessageRepository,
    C: ChannelRepository,
    P: StickerPackDirectory,
{
    pub fn new(
        report_repository: Arc<R>,
        user_repository: Arc<U>,
        community_repository: Arc<S>,
        message_repository: Arc<M>,
        dm_repository: Arc<D>,
        channel_repository: Arc<C>,
        sticker_packs: Arc<P>,
    ) -> Self {
        Self {
            report_repository,
            user_repository,
            community_repository,
            message_repository,
            dm_repository,
            channel_repository,
            sticker_packs,
        }
    }

    pub async fn execute(
        &self,
        reporter_id: UserId,
        request: CreateReportRequest,
    ) -> Result<(), CreateReportError> {
        let reason =
            ReportReason::parse(&request.reason).ok_or(CreateReportError::UnknownReason)?;
        let target_name = self
            .target_name(&reporter_id, request.target, request.target_id)
            .await?;
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
            ReportOutcome::EvidenceGone => Err(CreateReportError::ContextMismatch),
            ReportOutcome::AlreadyReported => Err(CreateReportError::AlreadyReported),
            ReportOutcome::LimitReached => Err(CreateReportError::TooManyReports),
        }
    }

    async fn target_name(
        &self,
        reporter_id: &UserId,
        target: ReportTarget,
        target_id: Uuid,
    ) -> Result<String, CreateReportError> {
        match target {
            ReportTarget::User => self
                .user_repository
                .get_name(&UserId::new(target_id))
                .await?
                .ok_or(CreateReportError::TargetNotFound),
            ReportTarget::Community => self
                .community_repository
                .find_community_by_id(&CommunityId::new(target_id))
                .await?
                .map(|community| community.name().as_str().to_string())
                .ok_or(CreateReportError::TargetNotFound),
            ReportTarget::StickerPack => {
                let pack = self
                    .sticker_packs
                    .find_pack(target_id)
                    .await
                    .map_err(|e| CreateReportError::Unavailable(e.0))?
                    .ok_or(CreateReportError::TargetNotFound)?;
                if &pack.owner_id == reporter_id {
                    return Err(ReportError::SelfReport.into());
                }
                Ok(pack.name)
            }
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
                let channel = match self.channel_repository.find_channel_by_id(&channel_id).await {
                    Ok(channel) => channel,
                    Err(RepositoryError::NotFound) => {
                        return Err(CreateReportError::ContextMismatch);
                    }
                    Err(e) => return Err(e.into()),
                };
                let belongs = match target {
                    ReportTarget::User => message.author_id().uuid() == target_id,
                    ReportTarget::Community => channel.community_id().uuid() == target_id,
                    ReportTarget::StickerPack => message.sticker().is_some_and(|sticker| sticker.pack_id == target_id),
                };
                if !belongs {
                    return Err(CreateReportError::ContextMismatch);
                }
                if !self
                    .community_repository
                    .is_community_member(channel.community_id(), reporter_id)
                    .await?
                {
                    return Err(CreateReportError::ContextMismatch);
                }
                if channel.visibility() == ChannelVisibility::Private
                    && !self
                        .channel_repository
                        .is_text_channel_member(&channel_id, reporter_id)
                        .await?
                {
                    return Err(CreateReportError::ContextMismatch);
                }
                return Ok(ReportContext {
                    channel_id: Some(channel_id.uuid()),
                    message_id: Some(message_id),
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
                let belongs = match target {
                    ReportTarget::User => {
                        message.sender_id().uuid() == target_id && message.recipient_id() == reporter_id
                    }
                    ReportTarget::StickerPack => {
                        message.sticker().is_some_and(|sticker| sticker.pack_id == target_id)
                            && (message.recipient_id() == reporter_id || message.sender_id() == reporter_id)
                    }
                    ReportTarget::Community => false,
                };
                if !belongs {
                    return Err(CreateReportError::ContextMismatch);
                }
                Ok(ReportContext {
                    channel_id: None,
                    message_id: Some(message_id),
                })
            }
            Err(RepositoryError::NotFound) => Err(CreateReportError::ContextMismatch),
            Err(e) => Err(e.into()),
        }
    }
}
