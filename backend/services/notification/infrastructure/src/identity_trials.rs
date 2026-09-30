use async_trait::async_trait;
use chrono::{DateTime, Utc};
use contracts::internal::users::TrialStateV1;
use domain::user::id::UserId;
use notification_core::ports::trial_state::TrialState;

pub struct IdentityTrialClient {
    client: reqwest::Client,
    base_url: String,
}

impl IdentityTrialClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }
}

#[async_trait]
impl TrialState for IdentityTrialClient {
    async fn active_trial_end(&self, user_id: &UserId) -> Result<Option<DateTime<Utc>>, String> {
        let response = self
            .client
            .get(format!("{}/internal/users/{}/trial", self.base_url, user_id.uuid()))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !response.status().is_success() {
            return Err(format!("identity trial lookup failed: {}", response.status()));
        }
        let state: TrialStateV1 = response.json().await.map_err(|e| e.to_string())?;
        Ok(state.valid_until.filter(|_| state.on_trial))
    }
}
