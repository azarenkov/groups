use async_trait::async_trait;
use crate::ports::server_owner_lookup::{ServerOwnerLookup, ServerOwnerLookupError};
use domain::{server::id::ServerId, user::id::UserId};
use contracts::internal::servers::OwnerResponseV1;

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
impl ServerOwnerLookup for MessagingOwnerClient {
    async fn find_owner(
        &self,
        server_id: &ServerId,
    ) -> Result<Option<UserId>, ServerOwnerLookupError> {
        let response = super::with_request_id(
            self.client
            .get(format!(
                "{}/internal/servers/{}/owner",
                self.base_url,
                server_id.uuid()
            )))
            .send()
            .await
            .map_err(|e| ServerOwnerLookupError::Unavailable(e.to_string()))?;
        match response.status().as_u16() {
            200 => {
                let dto: OwnerResponseV1 = response
                    .json()
                    .await
                    .map_err(|e| ServerOwnerLookupError::Unavailable(e.to_string()))?;
                Ok(Some(UserId::new(dto.owner_id)))
            }
            404 => Ok(None),
            status => Err(ServerOwnerLookupError::Unavailable(format!(
                "owner endpoint returned status {status}"
            ))),
        }
    }
}
