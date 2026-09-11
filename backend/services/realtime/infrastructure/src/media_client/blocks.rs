use async_trait::async_trait;
use contracts::internal::friendships::{BlockRelationsRequestV1, BlockRelationsResponseV1};
use realtime_core::ports::blocks::{BlockDirectory, BlockLookupError, BlockSets};
use uuid::Uuid;

pub struct MessagingBlockClient {
    client: reqwest::Client,
    base_url: String,
}

impl MessagingBlockClient {
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
impl BlockDirectory for MessagingBlockClient {
    async fn block_relations(&self, user_id: Uuid, candidates: &[Uuid]) -> Result<BlockSets, BlockLookupError> {
        if candidates.is_empty() {
            return Ok(BlockSets::default());
        }
        let response = self
            .client
            .post(format!("{}/internal/users/{}/block-relations", self.base_url, user_id))
            .json(&BlockRelationsRequestV1 { ids: candidates.to_vec() })
            .send()
            .await
            .map_err(|e| BlockLookupError::Unavailable(e.to_string()))?;
        if !response.status().is_success() {
            return Err(BlockLookupError::Unavailable(format!(
                "block-relations endpoint returned status {}",
                response.status()
            )));
        }
        let dto: BlockRelationsResponseV1 = response
            .json()
            .await
            .map_err(|e| BlockLookupError::Unavailable(e.to_string()))?;
        Ok(BlockSets {
            blocking: dto.blocking,
            blocked_by: dto.blocked_by,
        })
    }
}
