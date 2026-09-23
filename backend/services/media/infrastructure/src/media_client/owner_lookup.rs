use async_trait::async_trait;
use media::ports::community_owner_lookup::{CommunityOwnerLookup, CommunityOwnerLookupError};
use domain::{community::id::CommunityId, user::id::UserId};
use contracts::internal::communities::{CanManageResponseV1, OwnerResponseV1};

pub struct MessagingOwnerClient {
    client: reqwest::Client,
    base_url: String,
}

impl MessagingOwnerClient {
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
impl CommunityOwnerLookup for MessagingOwnerClient {
    async fn find_owner(
        &self,
        community_id: &CommunityId,
    ) -> Result<Option<UserId>, CommunityOwnerLookupError> {
        let response = super::with_request_id(
            self.client
            .get(format!(
                "{}/internal/communities/{}/owner",
                self.base_url,
                community_id.uuid()
            )))
            .send()
            .await
            .map_err(|e| CommunityOwnerLookupError::Unavailable(e.to_string()))?;
        match response.status().as_u16() {
            200 => {
                let dto: OwnerResponseV1 = response
                    .json()
                    .await
                    .map_err(|e| CommunityOwnerLookupError::Unavailable(e.to_string()))?;
                Ok(Some(UserId::new(dto.owner_id)))
            }
            404 => Ok(None),
            status => Err(CommunityOwnerLookupError::Unavailable(format!(
                "owner endpoint returned status {status}"
            ))),
        }
    }

    async fn can_manage(
        &self,
        community_id: &CommunityId,
        user_id: &UserId,
    ) -> Result<bool, CommunityOwnerLookupError> {
        let response = super::with_request_id(
            self.client.get(format!(
                "{}/internal/communities/{}/can-manage/{}",
                self.base_url,
                community_id.uuid(),
                user_id.uuid(),
            )),
        )
        .send()
        .await
        .map_err(|e| CommunityOwnerLookupError::Unavailable(e.to_string()))?;
        if response.status().as_u16() != 200 {
            return Err(CommunityOwnerLookupError::Unavailable(format!(
                "status {}",
                response.status()
            )));
        }
        let body: CanManageResponseV1 = response
            .json()
            .await
            .map_err(|e| CommunityOwnerLookupError::Unavailable(e.to_string()))?;
        Ok(body.allowed)
    }
}
