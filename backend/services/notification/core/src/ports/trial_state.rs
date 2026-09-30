use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::{email::Email, id::UserId, locale::Locale};

#[derive(Debug, Clone)]
pub struct ActiveTrial {
    pub ends_at: DateTime<Utc>,
    pub email: Email,
    pub locale: Locale,
}

#[async_trait]
pub trait TrialState: Send + Sync {
    async fn active_trial(&self, user_id: &UserId) -> Result<Option<ActiveTrial>, String>;
}
