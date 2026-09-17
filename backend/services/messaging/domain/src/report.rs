use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use uuid::Uuid;

pub const MAX_DETAILS_CHARS: usize = 1000;
pub const DAILY_REPORT_LIMIT: i64 = 20;
pub const DUPLICATE_WINDOW_HOURS: i64 = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportTarget {
    User,
    Server,
}

impl ReportTarget {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Server => "server",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "user" => Some(Self::User),
            "server" => Some(Self::Server),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportReason {
    Spam,
    Harassment,
    Hate,
    Threats,
    Scam,
    Nsfw,
    Csam,
    SelfHarm,
    Impersonation,
    Illegal,
    Violence,
    Raid,
    Other,
}

impl ReportReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Spam => "spam",
            Self::Harassment => "harassment",
            Self::Hate => "hate",
            Self::Threats => "threats",
            Self::Scam => "scam",
            Self::Nsfw => "nsfw",
            Self::Csam => "csam",
            Self::SelfHarm => "self_harm",
            Self::Impersonation => "impersonation",
            Self::Illegal => "illegal",
            Self::Violence => "violence",
            Self::Raid => "raid",
            Self::Other => "other",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "spam" => Some(Self::Spam),
            "harassment" => Some(Self::Harassment),
            "hate" => Some(Self::Hate),
            "threats" => Some(Self::Threats),
            "scam" => Some(Self::Scam),
            "nsfw" => Some(Self::Nsfw),
            "csam" => Some(Self::Csam),
            "self_harm" => Some(Self::SelfHarm),
            "impersonation" => Some(Self::Impersonation),
            "illegal" => Some(Self::Illegal),
            "violence" => Some(Self::Violence),
            "raid" => Some(Self::Raid),
            "other" => Some(Self::Other),
            _ => None,
        }
    }

    pub fn allowed_for(&self, target: ReportTarget) -> bool {
        match target {
            ReportTarget::User => matches!(
                self,
                Self::Spam
                    | Self::Harassment
                    | Self::Hate
                    | Self::Threats
                    | Self::Scam
                    | Self::Nsfw
                    | Self::Csam
                    | Self::SelfHarm
                    | Self::Impersonation
                    | Self::Illegal
                    | Self::Other
            ),
            ReportTarget::Server => matches!(
                self,
                Self::Spam
                    | Self::Harassment
                    | Self::Hate
                    | Self::Violence
                    | Self::Scam
                    | Self::Nsfw
                    | Self::Csam
                    | Self::Illegal
                    | Self::Raid
                    | Self::Other
            ),
        }
    }

    pub fn needs_priority_review(&self) -> bool {
        matches!(self, Self::Csam)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportStatus {
    Open,
    Reviewed,
    Rejected,
}

impl ReportStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Reviewed => "reviewed",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ReportContext {
    pub channel_id: Option<Uuid>,
    pub message_id: Option<Uuid>,
    pub message_content: Option<String>,
    pub attachment_ids: Vec<Uuid>,
}

#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error("Cannot report yourself")]
    SelfReport,
    #[error("Reason does not apply to this target")]
    ReasonNotAllowed,
    #[error("Details are required for this reason")]
    DetailsRequired,
    #[error("Details are too long")]
    DetailsTooLong,
}

#[derive(Debug, Clone)]
pub struct Report {
    id: Uuid,
    reporter_id: UserId,
    target: ReportTarget,
    target_id: Uuid,
    target_name: String,
    reason: ReportReason,
    details: Option<String>,
    context: ReportContext,
    status: ReportStatus,
    created_at: DateTime<Utc>,
}

impl Report {
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        reporter_id: UserId,
        target: ReportTarget,
        target_id: Uuid,
        target_name: String,
        reason: ReportReason,
        details: Option<String>,
        context: ReportContext,
        now: DateTime<Utc>,
    ) -> Result<Self, ReportError> {
        if target == ReportTarget::User && reporter_id.uuid() == target_id {
            return Err(ReportError::SelfReport);
        }
        if !reason.allowed_for(target) {
            return Err(ReportError::ReasonNotAllowed);
        }
        let details = details
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty());
        if reason == ReportReason::Other && details.is_none() {
            return Err(ReportError::DetailsRequired);
        }
        if details
            .as_ref()
            .is_some_and(|d| d.chars().count() > MAX_DETAILS_CHARS)
        {
            return Err(ReportError::DetailsTooLong);
        }
        Ok(Self {
            id: Uuid::new_v4(),
            reporter_id,
            target,
            target_id,
            target_name,
            reason,
            details,
            context,
            status: ReportStatus::Open,
            created_at: now,
        })
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn reporter_id(&self) -> &UserId {
        &self.reporter_id
    }

    pub fn target(&self) -> ReportTarget {
        self.target
    }

    pub fn target_id(&self) -> Uuid {
        self.target_id
    }

    pub fn target_name(&self) -> &str {
        &self.target_name
    }

    pub fn reason(&self) -> ReportReason {
        self.reason
    }

    pub fn details(&self) -> Option<&str> {
        self.details.as_deref()
    }

    pub fn context(&self) -> &ReportContext {
        &self.context
    }

    pub fn status(&self) -> ReportStatus {
        self.status
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportOutcome {
    Stored,
    EvidenceGone,
    AlreadyReported,
    LimitReached,
}

#[async_trait]
pub trait ReportRepository {
    async fn insert_within_quota(
        &self,
        report: &Report,
        since: DateTime<Utc>,
        daily_limit: i64,
    ) -> Result<ReportOutcome, RepositoryError>;
}

#[cfg(test)]
mod tests;
