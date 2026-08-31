use async_trait::async_trait;
use domain::user::id::UserId;
use ports::repository::RepositoryError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmPolicy {
    All,
    Friends,
    NoOne,
}

impl DmPolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Friends => "friends",
            Self::NoOne => "no_one",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "friends" => Some(Self::Friends),
            "no_one" => Some(Self::NoOne),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PrivacySettings {
    pub dm_policy: DmPolicy,
    pub accept_friend_requests: bool,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            dm_policy: DmPolicy::Friends,
            accept_friend_requests: true,
        }
    }
}

#[async_trait]
pub trait PrivacyRepository {
    async fn get(&self, user_id: &UserId) -> Result<PrivacySettings, RepositoryError>;
    async fn set(&self, user_id: &UserId, settings: PrivacySettings) -> Result<(), RepositoryError>;
}
