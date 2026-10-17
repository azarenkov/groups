use api_common::internal_client::{InternalClient, expect_json};
use async_trait::async_trait;
use contracts::internal::users::{ActivityAudienceRequestV1, ActivityAudienceResponseV1};
use domain::user::id::UserId;
use identity::ports::activity_audience::{ActivityAudience, ActivityAudienceError};

pub struct MessagingAudienceClient {
    messaging: InternalClient,
}

impl MessagingAudienceClient {
    pub fn new(base_url: &str) -> Self {
        Self { messaging: InternalClient::new(base_url) }
    }
}

#[async_trait]
impl ActivityAudience for MessagingAudienceClient {
    async fn visible_to(&self, viewer: &UserId, candidates: &[UserId]) -> Result<Vec<UserId>, ActivityAudienceError> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        let body = ActivityAudienceRequestV1 { ids: candidates.iter().map(|id| id.uuid()).collect() };
        let request = self
            .messaging
            .post(&format!("/internal/users/{}/activity-audience", viewer.uuid()))
            .json(&body);
        let dto: ActivityAudienceResponseV1 = expect_json(request, "activity-audience")
            .await
            .map_err(|e| ActivityAudienceError::Unavailable(e.to_string()))?;
        Ok(dto.ids.into_iter().map(UserId::new).collect())
    }
}
