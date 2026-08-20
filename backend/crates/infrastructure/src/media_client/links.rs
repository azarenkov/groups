use async_trait::async_trait;
use application::ports::attachment_links_lookup::{
    AttachmentLinksLookup, AttachmentLinksLookupError,
};
use contracts::internal::attachments::{LinkedRequestV1, LinkedResponseV1};
use uuid::Uuid;

pub struct MessagingLinksClient {
    client: reqwest::Client,
    base_url: String,
}

impl MessagingLinksClient {
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
impl AttachmentLinksLookup for MessagingLinksClient {
    async fn linked_subset(
        &self,
        ids: &[Uuid],
    ) -> Result<Vec<Uuid>, AttachmentLinksLookupError> {
        let response = self
            .client
            .post(format!("{}/internal/attachments/linked", self.base_url))
            .json(&LinkedRequestV1 {
                attachment_ids: ids.to_vec(),
            })
            .send()
            .await
            .map_err(|e| AttachmentLinksLookupError::Unavailable(e.to_string()))?;
        if !response.status().is_success() {
            return Err(AttachmentLinksLookupError::Unavailable(format!(
                "linked endpoint returned status {}",
                response.status()
            )));
        }
        let dto: LinkedResponseV1 = response
            .json()
            .await
            .map_err(|e| AttachmentLinksLookupError::Unavailable(e.to_string()))?;
        Ok(dto.linked_ids)
    }
}
