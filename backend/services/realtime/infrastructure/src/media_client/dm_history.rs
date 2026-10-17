use api_common::internal_client::{InternalClient, expect_json};
use async_trait::async_trait;
use contracts::internal::users::{DmPeersRequestV1, DmPeersResponseV1};
use realtime_core::ports::dm_history::{DmHistory, DmHistoryLookupError};
use uuid::Uuid;

pub struct MessagingDmHistoryClient {
    messaging: InternalClient,
}

impl MessagingDmHistoryClient {
    pub fn new(base_url: &str) -> Self {
        Self { messaging: InternalClient::new(base_url) }
    }
}

#[async_trait]
impl DmHistory for MessagingDmHistoryClient {
    async fn dm_peers(&self, user_id: Uuid, candidates: &[Uuid]) -> Result<Vec<Uuid>, DmHistoryLookupError> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        let request = self
            .messaging
            .post(&format!("/internal/users/{user_id}/dm-peers"))
            .json(&DmPeersRequestV1 { ids: candidates.to_vec() });
        let dto: DmPeersResponseV1 = expect_json(request, "dm-peers")
            .await
            .map_err(|e| DmHistoryLookupError::Unavailable(e.to_string()))?;
        Ok(dto.ids)
    }
}
