use async_trait::async_trait;
use contracts::internal::users::TrialStateV1;
use domain::user::id::UserId;
use domain::user::{email::Email, locale::Locale};
use notification_core::ports::trial_state::{ActiveTrial, TrialState};

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
    async fn active_trial(&self, user_id: &UserId) -> Result<Option<ActiveTrial>, String> {
        let response = api_common::middleware::internal::with_internal_token(
            self.client
                .get(format!("{}/internal/users/{}/trial", self.base_url, user_id.uuid())),
        )
        .send()
            .await
            .map_err(|e| e.to_string())?;
        if !response.status().is_success() {
            return Err(format!("identity trial lookup failed: {}", response.status()));
        }
        let state: TrialStateV1 = response.json().await.map_err(|e| e.to_string())?;
        if !state.on_trial {
            return Ok(None);
        }
        let (Some(ends_at), Some(email)) = (state.valid_until, state.email) else {
            return Ok(None);
        };
        Ok(Some(ActiveTrial {
            ends_at,
            email: Email::new(email),
            locale: state
                .locale
                .as_deref()
                .map(Locale::from_str_lenient)
                .unwrap_or_default(),
        }))
    }
}
