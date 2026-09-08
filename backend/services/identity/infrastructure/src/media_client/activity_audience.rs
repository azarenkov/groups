use async_trait::async_trait;
use contracts::internal::users::{ActivityAudienceRequestV1, ActivityAudienceResponseV1};
use domain::user::id::UserId;
use identity::ports::activity_audience::{ActivityAudience, ActivityAudienceError};

pub struct MessagingAudienceClient {
    client: reqwest::Client,
    base_url: String,
}

impl MessagingAudienceClient {
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
impl ActivityAudience for MessagingAudienceClient {
    async fn visible_to(
        &self,
        viewer: &UserId,
        candidates: &[UserId],
    ) -> Result<Vec<UserId>, ActivityAudienceError> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        let body = ActivityAudienceRequestV1 {
            ids: candidates.iter().map(|id| id.uuid()).collect(),
        };
        let response = super::with_request_id(
            self.client.post(format!(
                "{}/internal/users/{}/activity-audience",
                self.base_url,
                viewer.uuid()
            )),
        )
        .json(&body)
        .send()
        .await
        .map_err(|e| ActivityAudienceError::Unavailable(e.to_string()))?;
        if !response.status().is_success() {
            return Err(ActivityAudienceError::Unavailable(format!(
                "activity-audience endpoint returned status {}",
                response.status()
            )));
        }
        let dto: ActivityAudienceResponseV1 = response
            .json()
            .await
            .map_err(|e| ActivityAudienceError::Unavailable(e.to_string()))?;
        Ok(dto.ids.into_iter().map(UserId::new).collect())
    }
}
