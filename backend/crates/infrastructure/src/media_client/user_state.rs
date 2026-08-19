use async_trait::async_trait;
use application::ports::user_state_lookup::{UserStateLookup, UserStateLookupError};
use domain::user::id::UserId;
use serde::Deserialize;

pub struct MessagingStateClient {
    client: reqwest::Client,
    base_url: String,
}

impl MessagingStateClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StateDto {
    has_state: bool,
}

#[async_trait]
impl UserStateLookup for MessagingStateClient {
    async fn has_messaging_state(
        &self,
        user_id: &UserId,
    ) -> Result<bool, UserStateLookupError> {
        let response = self
            .client
            .get(format!(
                "{}/internal/users/{}/has-state",
                self.base_url,
                user_id.uuid()
            ))
            .send()
            .await
            .map_err(|e| UserStateLookupError::Unavailable(e.to_string()))?;
        if !response.status().is_success() {
            return Err(UserStateLookupError::Unavailable(format!(
                "has-state endpoint returned status {}",
                response.status()
            )));
        }
        let dto: StateDto = response
            .json()
            .await
            .map_err(|e| UserStateLookupError::Unavailable(e.to_string()))?;
        Ok(dto.has_state)
    }
}
