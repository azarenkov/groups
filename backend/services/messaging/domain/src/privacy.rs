use async_trait::async_trait;
use domain::user::id::UserId;
use ports::repository::RepositoryError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactPolicy {
    All,
    Friends,
    NoOne,
}

impl ContactPolicy {
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
    pub dm_policy: ContactPolicy,
    pub call_policy: ContactPolicy,
    pub accept_friend_requests: bool,
    pub show_local_time: bool,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            dm_policy: ContactPolicy::Friends,
            call_policy: ContactPolicy::Friends,
            accept_friend_requests: true,
            show_local_time: false,
        }
    }
}

#[async_trait]
pub trait PrivacyRepository {
    async fn get(&self, user_id: &UserId) -> Result<PrivacySettings, RepositoryError>;
    async fn apply_patch(
        &self,
        user_id: &UserId,
        dm_policy: Option<ContactPolicy>,
        call_policy: Option<ContactPolicy>,
        accept_friend_requests: Option<bool>,
        show_local_time: Option<bool>,
    ) -> Result<PrivacySettings, RepositoryError>;
}
