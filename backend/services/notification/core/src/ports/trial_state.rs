use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;

#[async_trait]
pub trait TrialState: Send + Sync {
    async fn active_trial_end(&self, user_id: &UserId) -> Result<Option<DateTime<Utc>>, String>;
}
