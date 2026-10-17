use api_common::internal_client::{InternalClient, expect_json};
use async_trait::async_trait;
use contracts::internal::attachments::{LinkedRequestV1, LinkedResponseV1};
use media::ports::attachment_links_lookup::{AttachmentLinksLookup, AttachmentLinksLookupError};
use uuid::Uuid;

pub struct MessagingLinksClient {
    messaging: InternalClient,
}

impl MessagingLinksClient {
    pub fn new(base_url: &str) -> Self {
        Self { messaging: InternalClient::new(base_url) }
    }
}

#[async_trait]
impl AttachmentLinksLookup for MessagingLinksClient {
    async fn linked_subset(&self, ids: &[Uuid]) -> Result<Vec<Uuid>, AttachmentLinksLookupError> {
        let request = self
            .messaging
            .post("/internal/attachments/linked")
            .json(&LinkedRequestV1 { attachment_ids: ids.to_vec() });
        let dto: LinkedResponseV1 = expect_json(request, "linked")
            .await
            .map_err(|e| AttachmentLinksLookupError::Unavailable(e.to_string()))?;
        Ok(dto.linked_ids)
    }
}
