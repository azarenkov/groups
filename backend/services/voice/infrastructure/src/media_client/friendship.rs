use api_common::internal_client::{InternalClient, block_relation, expect_json};
use async_trait::async_trait;
use contracts::internal::calls::CallPermitResponseV1;
use domain::user::id::UserId;
use voice::ports::friendship_lookup::{BlockRelation, FriendshipLookup, FriendshipLookupError};

pub struct MessagingFriendshipClient {
    messaging: InternalClient,
}

impl MessagingFriendshipClient {
    pub fn new(base_url: &str) -> Self {
        Self { messaging: InternalClient::new(base_url) }
    }
}

#[async_trait]
impl FriendshipLookup for MessagingFriendshipClient {
    async fn can_call(&self, caller: &UserId, callee: &UserId) -> Result<bool, FriendshipLookupError> {
        let request = self
            .messaging
            .get(&format!("/internal/calls/permit/{}/{}", caller.uuid(), callee.uuid()));
        let body: CallPermitResponseV1 = expect_json(request, "call permit")
            .await
            .map_err(|e| FriendshipLookupError::Unavailable(e.to_string()))?;
        Ok(body.allowed)
    }

    async fn block_relation(&self, a: &UserId, b: &UserId) -> Result<BlockRelation, FriendshipLookupError> {
        let body = block_relation(&self.messaging, a, b)
            .await
            .map_err(|e| FriendshipLookupError::Unavailable(e.to_string()))?;
        Ok(BlockRelation { blocking: body.blocking, blocked_by: body.blocked_by })
    }
}
