use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use uuid::Uuid;

pub const MAX_PINNED_MESSAGES: i64 = 50;
pub const MAX_PINNED_CONVERSATIONS: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinScope {
    Channel(Uuid),
    Dm { user_low: Uuid, user_high: Uuid },
}

impl PinScope {
    pub fn dm(a: &UserId, b: &UserId) -> Self {
        let (a, b) = (a.uuid(), b.uuid());
        Self::Dm {
            user_low: a.min(b),
            user_high: a.max(b),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinOutcome {
    Pinned(DateTime<Utc>),
    AlreadyPinned,
    LimitReached,
}

#[derive(Debug, Clone)]
pub struct PinnedMessage {
    pub message_id: Uuid,
    pub author_id: UserId,
    pub author_name: String,
    pub forwarded_author_id: Option<UserId>,
    pub content: String,
    pub sticker_emoji: Option<String>,
    pub has_attachments: bool,
    pub created_at: DateTime<Utc>,
    pub pinned_by: UserId,
    pub pinned_at: DateTime<Utc>,
}

#[async_trait]
pub trait MessagePinRepository: Send + Sync {
    async fn pin(
        &self,
        scope: PinScope,
        message_id: Uuid,
        pinned_by: &UserId,
        recipients: &[Uuid],
    ) -> Result<PinOutcome, RepositoryError>;

    async fn unpin(
        &self,
        scope: PinScope,
        message_id: Uuid,
        unpinned_by: &UserId,
        recipients: &[Uuid],
    ) -> Result<bool, RepositoryError>;

    async fn list(&self, scope: PinScope, viewer: &UserId) -> Result<Vec<PinnedMessage>, RepositoryError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConversationPinKind {
    Dm,
    Community,
}

impl ConversationPinKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "dm" | "dms" => Some(Self::Dm),
            "community" | "communities" => Some(Self::Community),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Dm => "dm",
            Self::Community => "community",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConversationPins {
    pub dms: Vec<Uuid>,
    pub communities: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversationPinOutcome {
    Updated(ConversationPins),
    LimitReached,
    UnknownTarget,
}

#[async_trait]
pub trait ConversationPinRepository: Send + Sync {
    async fn list(&self, user: &UserId) -> Result<ConversationPins, RepositoryError>;

    async fn pin(
        &self,
        user: &UserId,
        kind: ConversationPinKind,
        target: Uuid,
    ) -> Result<ConversationPinOutcome, RepositoryError>;

    async fn unpin(
        &self,
        user: &UserId,
        kind: ConversationPinKind,
        target: Uuid,
    ) -> Result<ConversationPins, RepositoryError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dm_scopes_do_not_depend_on_who_asks() {
        let a = UserId::new(Uuid::from_u128(2));
        let b = UserId::new(Uuid::from_u128(1));
        assert_eq!(PinScope::dm(&a, &b), PinScope::dm(&b, &a));
        assert_eq!(
            PinScope::dm(&a, &b),
            PinScope::Dm {
                user_low: Uuid::from_u128(1),
                user_high: Uuid::from_u128(2)
            }
        );
    }

    #[test]
    fn kinds_parse_both_forms() {
        assert_eq!(ConversationPinKind::parse("dms"), Some(ConversationPinKind::Dm));
        assert_eq!(ConversationPinKind::parse("community"), Some(ConversationPinKind::Community));
        assert_eq!(ConversationPinKind::parse("x"), None);
    }
}
