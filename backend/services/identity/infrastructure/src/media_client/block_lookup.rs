use async_trait::async_trait;
use contracts::internal::friendships::BlockRelationResponseV1;
use domain::user::id::UserId;
use identity::ports::block_lookup::{BlockLookup, BlockLookupError};

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
impl BlockLookup for MessagingBlockClient {
    async fn blocked_by(&self, viewer: &UserId, target: &UserId) -> Result<bool, BlockLookupError> {
        let response = super::with_request_id(self.client.get(format!(
            "{}/internal/blocks/{}/{}",
            self.base_url,
            viewer.uuid(),
            target.uuid()
        )))
        .send()
        .await
        .map_err(|e| BlockLookupError::Unavailable(e.to_string()))?;
        if !response.status().is_success() {
            return Err(BlockLookupError::Unavailable(format!(
                "block lookup endpoint returned status {}",
                response.status()
            )));
        }
        let dto: BlockRelationResponseV1 = response
            .json()
            .await
            .map_err(|e| BlockLookupError::Unavailable(e.to_string()))?;
        Ok(dto.blocked_by)
    }
}
