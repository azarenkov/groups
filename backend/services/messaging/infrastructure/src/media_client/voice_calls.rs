use async_trait::async_trait;
use contracts::internal::calls::CallParticipationResponseV1;
use domain::user::id::UserId;
use messaging::ports::call_participation::{CallParticipationError, CallParticipationLookup};
use uuid::Uuid;

pub struct VoiceCallsClient {
    client: reqwest::Client,
    base_url: String,
}

impl VoiceCallsClient {
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

#[async_trait]
impl CallParticipationLookup for VoiceCallsClient {
    async fn both_participated(
        &self,
        call_id: Uuid,
        a: &UserId,
        b: &UserId,
    ) -> Result<bool, CallParticipationError> {
        let response = super::with_request_id(self.client.get(format!(
            "{}/internal/calls/{}/participation/{}/{}",
            self.base_url,
            call_id,
            a.uuid(),
            b.uuid()
        )))
        .send()
        .await
        .map_err(|e| CallParticipationError::Unavailable(e.to_string()))?;
        if !response.status().is_success() {
            return Err(CallParticipationError::Unavailable(format!(
                "call participation endpoint returned status {}",
                response.status()
            )));
        }
        let body: CallParticipationResponseV1 = response
            .json()
            .await
            .map_err(|e| CallParticipationError::Unavailable(e.to_string()))?;
        Ok(body.participated)
    }
}
