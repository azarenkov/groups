use api_common::internal_client::{InternalClient, expect_json};
use async_trait::async_trait;
use contracts::internal::friendships::{BlockRelationsRequestV1, BlockRelationsResponseV1};
use contracts::internal::users::{DmPeersRequestV1, DmPeersResponseV1};
use realtime_core::ports::blocks::{BlockSets, RelationDirectory, RelationLookupError};
use uuid::Uuid;

pub struct MessagingRelationsClient {
    messaging: InternalClient,
}

impl MessagingRelationsClient {
    pub fn new(base_url: &str) -> Self {
        Self { messaging: InternalClient::new(base_url) }
    }
}

#[async_trait]
impl RelationDirectory for MessagingRelationsClient {
    async fn block_relations(&self, user_id: Uuid, candidates: &[Uuid]) -> Result<BlockSets, RelationLookupError> {
        if candidates.is_empty() {
            return Ok(BlockSets::default());
        }
        let request = self
            .messaging
            .post(&format!("/internal/users/{user_id}/block-relations"))
            .json(&BlockRelationsRequestV1 { ids: candidates.to_vec() });
        let dto: BlockRelationsResponseV1 = expect_json(request, "block-relations")
            .await
            .map_err(|e| RelationLookupError::Unavailable(e.to_string()))?;
        Ok(BlockSets { blocking: dto.blocking, blocked_by: dto.blocked_by })
    }

    async fn dm_peers(&self, user_id: Uuid, candidates: &[Uuid]) -> Result<Vec<Uuid>, RelationLookupError> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        let request = self
            .messaging
            .post(&format!("/internal/users/{user_id}/dm-peers"))
            .json(&DmPeersRequestV1 { ids: candidates.to_vec() });
        let dto: DmPeersResponseV1 = expect_json(request, "dm-peers")
            .await
            .map_err(|e| RelationLookupError::Unavailable(e.to_string()))?;
        Ok(dto.ids)
    }
}
