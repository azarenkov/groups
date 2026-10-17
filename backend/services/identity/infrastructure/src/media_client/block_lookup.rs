use api_common::internal_client::{InternalClient, block_relation};
use async_trait::async_trait;
use domain::user::id::UserId;
use identity::ports::block_lookup::{BlockLookup, BlockLookupError};

pub struct MessagingBlockClient {
    messaging: InternalClient,
}

impl MessagingBlockClient {
    pub fn new(base_url: &str) -> Self {
        Self { messaging: InternalClient::new(base_url) }
    }
}

#[async_trait]
impl BlockLookup for MessagingBlockClient {
    async fn blocked_by(&self, viewer: &UserId, target: &UserId) -> Result<bool, BlockLookupError> {
        let relation = block_relation(&self.messaging, viewer, target)
            .await
            .map_err(|e| BlockLookupError::Unavailable(e.to_string()))?;
        Ok(relation.blocked_by)
    }
}
