use async_trait::async_trait;
use contracts::internal::friendships::{BlockRelationsRequestV1, BlockRelationsResponseV1};
use contracts::internal::users::{DmPeersRequestV1, DmPeersResponseV1};
use realtime_core::ports::blocks::{RelationDirectory, RelationLookupError, BlockSets};
use uuid::Uuid;

pub struct MessagingRelationsClient {
    client: reqwest::Client,
    base_url: String,
}

impl MessagingRelationsClient {
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
impl RelationDirectory for MessagingRelationsClient {
    async fn block_relations(&self, user_id: Uuid, candidates: &[Uuid]) -> Result<BlockSets, RelationLookupError> {
        if candidates.is_empty() {
            return Ok(BlockSets::default());
        }
        let response = api_common::middleware::internal::with_internal_token(
            self.client
                .post(format!("{}/internal/users/{}/block-relations", self.base_url, user_id)),
        )
        .json(&BlockRelationsRequestV1 { ids: candidates.to_vec() })
        .send()
        .await
        .map_err(|e| RelationLookupError::Unavailable(e.to_string()))?;
        if !response.status().is_success() {
            return Err(RelationLookupError::Unavailable(format!(
                "block-relations endpoint returned status {}",
                response.status()
            )));
        }
        let dto: BlockRelationsResponseV1 = response
            .json()
            .await
            .map_err(|e| RelationLookupError::Unavailable(e.to_string()))?;
        Ok(BlockSets {
            blocking: dto.blocking,
            blocked_by: dto.blocked_by,
        })
    }

    async fn dm_peers(&self, user_id: Uuid, candidates: &[Uuid]) -> Result<Vec<Uuid>, RelationLookupError> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        let response = api_common::middleware::internal::with_internal_token(
            self.client
                .post(format!("{}/internal/users/{}/dm-peers", self.base_url, user_id)),
        )
        .json(&DmPeersRequestV1 { ids: candidates.to_vec() })
        .send()
        .await
        .map_err(|e| RelationLookupError::Unavailable(e.to_string()))?;
        if !response.status().is_success() {
            return Err(RelationLookupError::Unavailable(format!(
                "dm-peers endpoint returned status {}",
                response.status()
            )));
        }
        let dto: DmPeersResponseV1 = response
            .json()
            .await
            .map_err(|e| RelationLookupError::Unavailable(e.to_string()))?;
        Ok(dto.ids)
    }
}
