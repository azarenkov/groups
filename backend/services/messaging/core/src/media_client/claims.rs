use async_trait::async_trait;
use crate::ports::attachment_claims::{AttachmentClaimError, AttachmentClaims};
use contracts::internal::attachments::{ClaimRequestV1, ClaimedAttachmentV1, ReleaseRequestV1};
use domain::{attachment::AttachmentLink, user::id::UserId};
use uuid::Uuid;

pub struct MediaClaimsClient {
    client: reqwest::Client,
    base_url: String,
}

impl MediaClaimsClient {
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

fn to_link(d: ClaimedAttachmentV1) -> AttachmentLink {
    AttachmentLink {
        id: d.id,
        file_name: d.file_name,
        content_type: d.content_type,
        size_bytes: d.size_bytes,
        storage_key: d.storage_key,
        created_at: d.created_at,
    }
}

#[async_trait]
impl AttachmentClaims for MediaClaimsClient {
    async fn claim(
        &self,
        ids: &[Uuid],
        owner_id: &UserId,
        claim_token: Uuid,
    ) -> Result<Vec<AttachmentLink>, AttachmentClaimError> {
        let response = super::with_request_id(
            self.client
            .post(format!("{}/internal/attachments/claim", self.base_url))
            .json(&ClaimRequestV1 {
                attachment_ids: ids.to_vec(),
                owner_id: owner_id.uuid(),
                claim_token,
            }))
            .send()
            .await
            .map_err(|e| AttachmentClaimError::Unavailable(e.to_string()))?;

        match response.status().as_u16() {
            200 => {
                let items: Vec<ClaimedAttachmentV1> = response
                    .json()
                    .await
                    .map_err(|e| AttachmentClaimError::Unavailable(e.to_string()))?;
                Ok(items.into_iter().map(to_link).collect())
            }
            422 => Err(AttachmentClaimError::Invalid),
            status => Err(AttachmentClaimError::Unavailable(format!(
                "claim endpoint returned status {status}"
            ))),
        }
    }

    async fn lookup_unclaimed(
        &self,
        id: Uuid,
        owner_id: &UserId,
    ) -> Result<Option<AttachmentLink>, AttachmentClaimError> {
        let response = super::with_request_id(
            self.client
            .get(format!(
                "{}/internal/attachments/{}/unclaimed?ownerId={}",
                self.base_url,
                id,
                owner_id.uuid()
            )))
            .send()
            .await
            .map_err(|e| AttachmentClaimError::Unavailable(e.to_string()))?;
        match response.status().as_u16() {
            200 => {
                let dto: ClaimedAttachmentV1 = response
                    .json()
                    .await
                    .map_err(|e| AttachmentClaimError::Unavailable(e.to_string()))?;
                Ok(Some(to_link(dto)))
            }
            404 => Ok(None),
            status => Err(AttachmentClaimError::Unavailable(format!(
                "unclaimed lookup returned status {status}"
            ))),
        }
    }

    async fn release(&self, ids: &[Uuid], claim_token: Uuid) -> Result<(), AttachmentClaimError> {
        let response = super::with_request_id(
            self.client
            .post(format!("{}/internal/attachments/release", self.base_url))
            .json(&ReleaseRequestV1 {
                attachment_ids: ids.to_vec(),
                claim_token,
            }))
            .send()
            .await
            .map_err(|e| AttachmentClaimError::Unavailable(e.to_string()))?;
        if !response.status().is_success() {
            return Err(AttachmentClaimError::Unavailable(format!(
                "release endpoint returned status {}",
                response.status()
            )));
        }
        Ok(())
    }
}
