use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClearScope {
    Me,
    Everyone,
}

impl ClearScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Me => "me",
            Self::Everyone => "everyone",
        }
    }
}

#[async_trait]
pub trait DmConversationRepository: Send + Sync {
    async fn clear(
        &self,
        actor: &UserId,
        peer: &UserId,
        scope: ClearScope,
    ) -> Result<DateTime<Utc>, RepositoryError>;

    async fn set_hidden(&self, user: &UserId, peer: &UserId, hidden: bool) -> Result<(), RepositoryError>;

    async fn list_hidden(&self, user: &UserId) -> Result<Vec<Uuid>, RepositoryError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_scope_wire_names() {
        assert_eq!(ClearScope::Me.as_str(), "me");
        assert_eq!(ClearScope::Everyone.as_str(), "everyone");
    }
}
