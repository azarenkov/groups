use api_common::internal_client::{InternalCallError, InternalClient, expect_json, read_json, send};
use async_trait::async_trait;
use contracts::internal::communities::{CanManageResponseV1, OwnerResponseV1};
use domain::{community::id::CommunityId, user::id::UserId};
use chrono::{DateTime, Utc};
use media::ports::community_owner_lookup::{CommunityOwner, CommunityOwnerLookup, CommunityOwnerLookupError};

pub struct MessagingOwnerClient {
    messaging: InternalClient,
}

impl MessagingOwnerClient {
    pub fn new(base_url: &str) -> Self {
        Self { messaging: InternalClient::new(base_url) }
    }
}

fn unavailable(error: InternalCallError) -> CommunityOwnerLookupError {
    CommunityOwnerLookupError::Unavailable(error.to_string())
}

#[async_trait]
impl CommunityOwnerLookup for MessagingOwnerClient {
    async fn find_owner(&self, community_id: &CommunityId) -> Result<Option<CommunityOwner>, CommunityOwnerLookupError> {
        let request = self.messaging.get(&format!("/internal/communities/{}/owner", community_id.uuid()));
        let response = send(request).await.map_err(unavailable)?;
        match response.status().as_u16() {
            200 => {
                let dto: OwnerResponseV1 = read_json(response).await.map_err(unavailable)?;
                Ok(Some(CommunityOwner { user_id: UserId::new(dto.owner_id), as_of: dto.as_of.unwrap_or(DateTime::<Utc>::UNIX_EPOCH) }))
            }
            404 => Ok(None),
            status => Err(unavailable(InternalCallError::Status { endpoint: "owner", status })),
        }
    }

    async fn can_manage(&self, community_id: &CommunityId, user_id: &UserId) -> Result<bool, CommunityOwnerLookupError> {
        let request = self
            .messaging
            .get(&format!("/internal/communities/{}/can-manage/{}", community_id.uuid(), user_id.uuid()));
        let body: CanManageResponseV1 = expect_json(request, "can-manage").await.map_err(unavailable)?;
        Ok(body.allowed)
    }
}
