use async_trait::async_trait;
use application::ports::attachment_claims::{AttachmentClaimError, AttachmentClaims};
use chrono::{DateTime, Utc};
use domain::{attachment::AttachmentLink, user::id::UserId};
use serde::Deserialize;
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaimedAttachmentDto {
    id: Uuid,
    file_name: String,
    content_type: String,
    size_bytes: i64,
    storage_key: String,
    created_at: DateTime<Utc>,
}

impl From<ClaimedAttachmentDto> for AttachmentLink {
    fn from(d: ClaimedAttachmentDto) -> Self {
        Self {
            id: d.id,
            file_name: d.file_name,
            content_type: d.content_type,
            size_bytes: d.size_bytes,
            storage_key: d.storage_key,
            created_at: d.created_at,
        }
    }
}

#[async_trait]
impl AttachmentClaims for MediaClaimsClient {
    async fn claim(
        &self,
        ids: &[Uuid],
        owner_id: &UserId,
    ) -> Result<Vec<AttachmentLink>, AttachmentClaimError> {
        let response = self
            .client
            .post(format!("{}/internal/attachments/claim", self.base_url))
            .json(&serde_json::json!({
                "attachmentIds": ids,
                "ownerId": owner_id.uuid(),
            }))
            .send()
            .await
            .map_err(|e| AttachmentClaimError::Unavailable(e.to_string()))?;

        match response.status().as_u16() {
            200 => {
                let items: Vec<ClaimedAttachmentDto> = response
                    .json()
                    .await
                    .map_err(|e| AttachmentClaimError::Unavailable(e.to_string()))?;
                Ok(items.into_iter().map(AttachmentLink::from).collect())
            }
            422 => Err(AttachmentClaimError::Invalid),
            status => Err(AttachmentClaimError::Unavailable(format!(
                "claim endpoint returned status {status}"
            ))),
        }
    }

    async fn release(&self, ids: &[Uuid]) -> Result<(), AttachmentClaimError> {
        let response = self
            .client
            .post(format!("{}/internal/attachments/release", self.base_url))
            .json(&serde_json::json!({ "attachmentIds": ids }))
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
